use crate::lterm::{LTerm, LTermInner};
use crate::lvalue::LValue;
use crate::user::UserState;
use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::Rc;

mod substitution;
pub use substitution::SMap;

mod unification;
use unification::unify_rec;

pub mod constraint;
pub use constraint::{
    BaseConstraint, Constraint, DiseqFdConstraint, DisequalityConstraint, DistinctFdConstraint,
    FiniteDomain, LessThanOrEqualFdConstraint, MinusFdConstraint, PlusFdConstraint,
    PlusZConstraint, TimesFdConstraint, TimesZConstraint, TreeConstraint, UserConstraint,
};

use constraint::store::ConstraintStore;

mod reification;
pub use reification::reify;

pub type SResult<U> = Result<State<U>, ()>;

/// Logic program state
///
/// The `State` structure represents a state of the search. A logic program consists of goals,
/// which when applied to states, produce streams of states. Each state is a solution to a
/// (part of) logic program. The `State` can be cloned and each clone can be modified independently
/// of each other; the data structures within `State` are clone-on-write.
///
/// A state has four separate data storages that are clone-on-write:
///    1. The current substitution of LTerms
///    2. The constraint store
///    3. The domain store
///    4. User data
#[derive(Debug, Clone)]
pub struct State<U: UserState> {
    /// The substitution map
    smap: Rc<SMap>,

    /// The constraint store
    cstore: Rc<ConstraintStore<U>>,

    /// The domain store
    dstore: Rc<HashMap<LTerm, Rc<FiniteDomain>>>,

    pub user_state: U,
}

impl<U: UserState> State<U> {
    pub fn new(user_state: U) -> State<U> {
        State {
            smap: Rc::new(SMap::new()),
            cstore: Rc::new(ConstraintStore::new()),
            dstore: Rc::new(HashMap::new()),
            user_state,
        }
    }

    /// Return a reference to the substition map of the state
    pub fn smap_ref(&self) -> &SMap {
        self.smap.as_ref()
    }

    pub fn smap_to_mut(&mut self) -> &mut SMap {
        Rc::make_mut(&mut self.smap)
    }

    /// Returns the state with replaced substitution map
    pub fn with_smap(self, smap: SMap) -> State<U> {
        State {
            smap: Rc::new(smap),
            ..self
        }
    }

    /// Get a cloned reference to the substitution map of the state
    pub fn get_smap(&self) -> Rc<SMap> {
        Rc::clone(&self.smap)
    }

    /// Return a reference to the constraint store of the state
    pub fn cstore_ref(&self) -> &ConstraintStore<U> {
        self.cstore.as_ref()
    }

    pub fn cstore_to_mut(&mut self) -> &mut ConstraintStore<U> {
        Rc::make_mut(&mut self.cstore)
    }

    /// Returns the state with replaced with a new constraint store. The old store is dropped.
    pub fn with_cstore(self, cstore: ConstraintStore<U>) -> State<U> {
        State {
            cstore: Rc::new(cstore),
            ..self
        }
    }

    pub fn get_cstore(&self) -> Rc<ConstraintStore<U>> {
        Rc::clone(&self.cstore)
    }

    /// Return a reference to the domain store of the state
    pub fn dstore_ref(&self) -> &HashMap<LTerm, Rc<FiniteDomain>> {
        self.dstore.as_ref()
    }

    pub fn dstore_to_mut(&mut self) -> &mut HashMap<LTerm, Rc<FiniteDomain>> {
        Rc::make_mut(&mut self.dstore)
    }

    pub fn with_dstore(self, dstore: HashMap<LTerm, Rc<FiniteDomain>>) -> State<U> {
        State {
            dstore: Rc::new(dstore),
            ..self
        }
    }

    /// Get a cloned reference to the domain store fo the state
    pub fn get_dstore(&self) -> Rc<HashMap<LTerm, Rc<FiniteDomain>>> {
        Rc::clone(&self.dstore)
    }

    /// Return the state with a new constraint
    pub fn with_constraint<T: Into<Constraint<U>>>(mut self, constraint: T) -> State<U> {
        self.cstore_to_mut().push_and_normalize(constraint.into());
        self
    }

    pub fn take_constraint(
        mut self,
        constraint: &Constraint<U>,
    ) -> (State<U>, Option<Constraint<U>>) {
        match self.cstore_to_mut().take(constraint) {
            Some(constraint) => (self, Some(constraint)),
            None => (self, None),
        }
    }

    /// Adds a new domain constraint for a variable `x`; or if the term is a value, then
    /// checks that the value is within the domain. If new domain constraint is added for a
    /// variable, it is updated to the domain store.
    pub fn process_domain(self, x: &LTerm, domain: Rc<FiniteDomain>) -> SResult<U> {
        match x.as_ref() {
            LTermInner::Var(_, _) => self.update_var_domain(x, domain),
            LTermInner::Val(LValue::Number(v)) if domain.contains(*v) => Ok(self),
            _ => Err(()),
        }
    }

    /// Updates domain constraint of a variable `x`.
    ///
    /// If variable does not have an existing domain, then it is given the `domain`.
    ///
    /// If the variable `x` is already constrained, then the resulting constraint is such that it
    /// fulfills both the old and the new constraint; i.e. it is an intersection of the domains.
    /// If the domains are disjoint, the constraint fails and `None` is returned.
    ///
    /// Note: if domains are resolved into singletons, then they are converted into value
    ///       kind LTerms.
    fn update_var_domain(self, x: &LTerm, domain: Rc<FiniteDomain>) -> SResult<U> {
        assert!(x.is_var());
        match self.dstore.get(x) {
            Some(old_domain) => match old_domain.intersect(domain.as_ref()) {
                Some(intersection) => self.resolve_storable_domain(x, Rc::new(intersection)),
                None => Err(()), /* disjoint domains */
            },
            None => self.resolve_storable_domain(x, domain),
        }
    }

    /// Stores a new `domain` for a variable `x` by updating the corresponding domain information
    /// of the state. Any existing domain information is replaced with the new.
    ///
    /// If the domain is a singleton, i.e. a single value, it is converted into a constant value
    /// instead, by creating a new constant from the singleton value and extending the
    /// substitution to map from the variable `x` to the newly created constant.
    fn resolve_storable_domain(mut self, x: &LTerm, domain: Rc<FiniteDomain>) -> SResult<U> {
        assert!(x.is_var());
        match domain.singleton_value() {
            Some(n) => {
                // Extend substitution from `x` to the singleton value `n`
                self.smap_to_mut().extend(x.clone(), LTerm::from(n));

                // Remove domain information from store
                let _ = self.dstore_to_mut().remove(x);

                // The substitution has been modified, re-run constraints.
                self.run_constraints()
            }
            None => {
                // Extend or update domain store with the given `domain`
                let _ = self.dstore_to_mut().insert(x.clone(), domain);
                Ok(self)
            }
        }
    }

    pub fn remove_domain(mut self, x: &LTerm) -> SResult<U> {
        match self.dstore_to_mut().remove(x) {
            Some(_) => Ok(self),
            None => Err(()),
        }
    }

    // Removes domain `exclude` from the domain of all variables in list `x`.
    pub fn exclude_from_domain(mut self, x: &LTerm, exclude: Rc<FiniteDomain>) -> SResult<U> {
        assert!(x.is_list());
        let dstore = self.get_dstore();
        for y in x {
            match dstore.get(&y) {
                Some(domain) => {
                    match self.process_domain(&y, Rc::new(domain.diff(exclude.as_ref()).ok_or(())?))
                    {
                        Ok(state) => self = state,
                        Err(error) => return Err(error),
                    }
                }
                None => (),
            }
        }
        Ok(self)
    }

    /// Runs all constraints from the constraint store on the current state. If any of the
    /// constraints fail, `None` is returned. Otherwise the state is returned with an updated
    /// constraint store.
    pub fn run_constraints(mut self) -> SResult<U> {
        let mut constraints = self.cstore.iter().cloned().collect::<Vec<Constraint<U>>>();

        // Each constraint is first removed from the store and then run against the state.
        // If the constraint does not want to be removed from the store, it adds itself
        // back when it is run.
        for constraint in constraints.drain(..) {
            self = match self.take_constraint(&constraint) {
                (unconstrained_state, Some(constraint)) => {
                    match constraint.run(unconstrained_state) {
                        Ok(constrained_state) => constrained_state,
                        Err(error) => return Err(error),
                    }
                }
                (constrained_state, None) => constrained_state, /* Constraint has removed itself. */
            };
        }

        Ok(self)
    }

    /// Processes extension for disequality constraints.
    fn process_extension_diseq(self, _extension: &SMap) -> SResult<U> {
        self.run_constraints()
    }

    /// Processes extension for finite domain constraints.
    ///
    /// For each new substitution we need to add a new domain constraint. If variable `x`
    /// is substituted with term `v`, then `x` and `v` must have domains with non-zero
    /// intersection. If variable `x` has a domain, then the domain is assigned to the
    /// term `v` as well.
    ///
    /// If all substitutions are successful domain constraints, a state with updated
    /// domain- and constraint-stores is returned.
    ///
    /// If the resulting intersection domain is non-zero, the
    /// substitution is not possible, the constraint fails and `None` is returned.
    fn process_extension_fd(mut self, extension: &SMap) -> SResult<U> {
        let dstore = self.get_dstore();
        for (x, v) in extension.iter() {
            match dstore.get(x) {
                Some(domain) => {
                    self = self
                        .process_domain(v, domain.clone())?
                        .remove_domain(x)?
                        .run_constraints()?
                }
                None => {
                    // No domain information found in store for `x`.
                }
            }
        }
        Ok(self)
    }

    fn process_extension_user(self, extension: &SMap) -> SResult<U> {
        UserState::process_extension(self, extension)
    }

    /// Processes the extension to substitution
    ///
    /// The extension to substitution consists of all substitutions added in a single
    /// unification. It consists of the substitutions had to be added in order to unify
    /// the two terms.
    fn process_extension(self, extension: SMap) -> SResult<U> {
        self.process_extension_diseq(&extension)?
            .process_extension_fd(&extension)?
            .process_extension_user(&extension)
    }

    /// Verifies that all variables constrained by domain constraints have domains
    /// associated with them.
    pub fn verify_all_bound(&self) {
        for constraint in self.cstore_ref().iter().filter(|c| c.is_finite_domain()) {
            for u in &constraint.operands() {
                let uwalk = self.smap_ref().walk(u);
                if uwalk.is_var() && !self.dstore_ref().contains_key(uwalk) {
                    panic!(
                        "Error: Variable {:?} not bound to any domain. {:?}",
                        u, self
                    );
                }
            }
        }
    }

    pub fn unify(mut self, u: &LTerm, v: &LTerm) -> SResult<U> {
        // Extension will contain all substitutions added in the recursive unification of the terms
        let mut extension = SMap::new();
        if unify_rec(&mut self.smap, &mut extension, u, v) {
            self.process_extension(extension)
        } else {
            Err(())
        }
    }

    /// Add disequality constraint
    pub fn disunify(self, u: &LTerm, v: &LTerm) -> SResult<U> {
        // Disunification is implemented in terms of unification
        let mut extension = SMap::new();
        let mut state = self.clone();
        if unify_rec(&mut state.smap, &mut extension, u, v) {
            if extension.is_empty() {
                // Unification succeeded without extending the current substitution, therefore
                // disequality constraint fails.
                Err(())
            } else {
                // Unification succeeded with extended substitution map. Instead of adding the
                // substitutions to the state, we add corresponding constraint to disequality
                // constraint store, against which later unifications will be verified.
                let c = Rc::new(DisequalityConstraint::from(extension));
                Ok(self.with_constraint(c))
            }
        } else {
            Ok(self)
        }
    }

    pub fn reify(&mut self) {
        let cstore = self.get_cstore();
        for c in cstore.iter() {
            c.reify(self);
        }
        U::reify(self);
    }
}
