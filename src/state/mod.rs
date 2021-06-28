use crate::lterm::{LTerm, LTermInner};
use crate::lvalue::LValue;
use crate::relation::diseq::DisequalityConstraint;
use crate::user::{User, DefaultUser};
use crate::engine::{Engine, DefaultEngine};
use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::Rc;

mod substitution;
pub use substitution::SMap;

mod unification;
pub use unification::unify_rec;

pub mod constraint;
pub use constraint::Constraint;

pub mod fd;
pub use fd::FiniteDomain;

use constraint::store::ConstraintStore;

mod reification;
pub use reification::reify;

pub type SResult<U, E> = Result<State<U, E>, ()>;

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
#[derive(Derivative, Debug)]
#[derivative(Clone(bound="U: User"))]
pub struct State<U = DefaultUser, E = DefaultEngine<DefaultUser>>
where
    U: User,
    E: Engine<U>,
{
    /// The substitution map
    pub smap: Rc<SMap<U, E>>,

    /// The constraint store
    cstore: Rc<ConstraintStore<U, E>>,

    /// The domain store
    dstore: Rc<HashMap<LTerm<U, E>, Rc<FiniteDomain>>>,

    pub user_state: U,
}

impl<U, E> State<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(user_state: U) -> State<U, E> {
        State {
            smap: Rc::new(SMap::new()),
            cstore: Rc::new(ConstraintStore::new()),
            dstore: Rc::new(HashMap::new()),
            user_state,
        }
    }

    /// Return a reference to the substition map of the state
    pub fn smap_ref(&self) -> &SMap<U, E> {
        self.smap.as_ref()
    }

    pub fn smap_to_mut(&mut self) -> &mut SMap<U, E> {
        Rc::make_mut(&mut self.smap)
    }

    /// Returns the state with replaced substitution map
    pub fn with_smap(self, smap: SMap<U, E>) -> State<U, E> {
        State {
            smap: Rc::new(smap),
            ..self
        }
    }

    /// Get a cloned reference to the substitution map of the state
    pub fn get_smap(&self) -> Rc<SMap<U, E>> {
        Rc::clone(&self.smap)
    }

    /// Return a reference to the constraint store of the state
    pub fn cstore_ref(&self) -> &ConstraintStore<U, E> {
        self.cstore.as_ref()
    }

    pub fn cstore_to_mut(&mut self) -> &mut ConstraintStore<U, E> {
        Rc::make_mut(&mut self.cstore)
    }

    /// Returns the state with replaced with a new constraint store. The old store is dropped.
    pub fn with_cstore(mut self, cstore: ConstraintStore<U, E>) -> State<U, E> {
        let old_cstore = self.get_cstore();
        for c in old_cstore.iter() {
            self = self.take_constraint(c).0;
        }
        for c in cstore.into_iter() {
            self = self.with_constraint(c)
        }
        self
    }

    pub fn get_cstore(&self) -> Rc<ConstraintStore<U, E>> {
        Rc::clone(&self.cstore)
    }

    /// Return a reference to the domain store of the state
    pub fn dstore_ref(&self) -> &HashMap<LTerm<U, E>, Rc<FiniteDomain>> {
        self.dstore.as_ref()
    }

    pub fn dstore_to_mut(&mut self) -> &mut HashMap<LTerm<U, E>, Rc<FiniteDomain>> {
        Rc::make_mut(&mut self.dstore)
    }

    pub fn with_dstore(self, dstore: HashMap<LTerm<U, E>, Rc<FiniteDomain>>) -> State<U, E> {
        State {
            dstore: Rc::new(dstore),
            ..self
        }
    }

    /// Get a cloned reference to the domain store fo the state
    pub fn get_dstore(&self) -> Rc<HashMap<LTerm<U, E>, Rc<FiniteDomain>>> {
        Rc::clone(&self.dstore)
    }

    /// Return the state with a new constraint
    pub fn with_constraint(mut self, constraint: Rc<dyn Constraint<U, E>>) -> State<U, E> {
        U::with_constraint(&mut self, &constraint);
        self.cstore_to_mut().push_and_normalize(constraint);
        self
    }

    pub fn take_constraint(
        mut self,
        constraint: &Rc<dyn Constraint<U, E>>,
    ) -> (State<U, E>, Option<Rc<dyn Constraint<U, E>>>) {
        match self.cstore_to_mut().take(constraint) {
            Some(constraint) => {
                U::take_constraint(&mut self, &constraint);
                (self, Some(constraint))
            }
            None => (self, None),
        }
    }

    /// Adds a new domain constraint for a variable `x`; or if the term is a value, then
    /// checks that the value is within the domain. If new domain constraint is added for a
    /// variable, it is updated to the domain store.
    pub fn process_domain(self, x: &LTerm<U, E>, domain: Rc<FiniteDomain>) -> SResult<U, E> {
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
    fn update_var_domain(self, x: &LTerm<U, E>, domain: Rc<FiniteDomain>) -> SResult<U, E> {
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
    fn resolve_storable_domain(mut self, x: &LTerm<U, E>, domain: Rc<FiniteDomain>) -> SResult<U, E> {
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

    pub fn remove_domain(mut self, x: &LTerm<U, E>) -> SResult<U, E> {
        match self.dstore_to_mut().remove(x) {
            Some(_) => Ok(self),
            None => Err(()),
        }
    }

    // Removes domain `exclude` from the domain of all variables in list `x`.
    pub fn exclude_from_domain(mut self, x: &LTerm<U, E>, exclude: Rc<FiniteDomain>) -> SResult<U, E> {
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
    pub fn run_constraints(mut self) -> SResult<U, E> {
        let mut constraints = self
            .cstore
            .iter()
            .cloned()
            .collect::<Vec<Rc<dyn Constraint<U, E>>>>();

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
    fn process_extension_diseq(self, _extension: &SMap<U, E>) -> SResult<U, E> {
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
    fn process_extension_fd(mut self, extension: &SMap<U, E>) -> SResult<U, E> {
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

    fn process_extension_user(self, extension: &SMap<U, E>) -> SResult<U, E> {
        User::process_extension(self, extension)
    }

    /// Processes the extension to substitution
    ///
    /// The extension to substitution consists of all substitutions added in a single
    /// unification. It consists of the substitutions had to be added in order to unify
    /// the two terms.
    fn process_extension(self, extension: SMap<U, E>) -> SResult<U, E> {
        self.process_extension_diseq(&extension)?
            .process_extension_fd(&extension)?
            .process_extension_user(&extension)
    }

    fn is_finite_domain(constraint: &Rc<dyn Constraint<U, E>>) -> bool {
        constraint.is::<crate::relation::ltefd::LessThanOrEqualFdConstraint<U, E>>()
            || constraint.is::<crate::relation::plusfd::PlusFdConstraint<U, E>>()
            || constraint.is::<crate::relation::minusfd::MinusFdConstraint<U, E>>()
            || constraint.is::<crate::relation::timesfd::TimesFdConstraint<U, E>>()
            || constraint.is::<crate::relation::diseqfd::DiseqFdConstraint<U, E>>()
            || constraint.is::<crate::relation::distinctfd::DistinctFdConstraint<U, E>>()
            || constraint.is::<crate::relation::distinctfd::DistinctFd2Constraint<U, E>>()
    }

    /// Verifies that all variables constrained by domain constraints have domains
    /// associated with them.
    pub fn verify_all_bound(&self) {
        for constraint in self
            .cstore_ref()
            .iter()
            .filter(|c| State::is_finite_domain(c))
        {
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

    pub fn unify(self, u: &LTerm<U, E>, v: &LTerm<U, E>) -> SResult<U, E> {
        // Extension will contain all substitutions added in the recursive unification of the terms
        let mut extension = SMap::new();
        unify_rec(self, &mut extension, u, v)?.process_extension(extension)
    }

    /// Add disequality constraint
    pub fn disunify(self, u: &LTerm<U, E>, v: &LTerm<U, E>) -> SResult<U, E> {
        // Disunification is implemented in terms of unification
        let mut extension = SMap::new();
        match unify_rec(self.clone(), &mut extension, u, v) {
            Ok(_) => {
                if extension.is_empty() {
                    // Unification succeeded without extending the current substitution, therefore
                    // disequality constraint fails.
                    Err(())
                } else {
                    // Unification succeeded with extended substitution map. Instead of adding the
                    // substitutions to the state, we add corresponding constraint to disequality
                    // constraint store, against which later unifications will be verified.
                    let c = DisequalityConstraint::new(extension);
                    Ok(self.with_constraint(c))
                }
            }
            Err(_) => Ok(self),
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
