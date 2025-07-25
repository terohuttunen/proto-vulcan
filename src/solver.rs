use crate::engine::{DefaultEngine, Engine};
use crate::goal::{DFSGoal, Goal};
use crate::state::State;
use crate::stream::{LazyStream, Stream};
use crate::user::{DefaultUser, User};
use std::any::{Any, TypeId};
use std::fmt;
use std::rc::Rc;

#[cfg(feature = "debugger")]
use crate::debugger::Debugger;

/// Result of a solver iteration
#[derive(Debug)]
pub enum SolverResult {
    /// Found a solution state
    Solution(Box<State>),
    /// No more solutions available (natural completion)
    NoMoreSolutions,
    /// Execution timed out
    Timeout,
    /// An error occurred during execution
    Error(String),
}

/// Check timeout every N iterations to reduce overhead
const TIMEOUT_CHECK_FREQUENCY: usize = 100;

pub struct Solver {
    context: <DefaultUser as User>::UserContext,
    engine: DefaultEngine,
    stream_iter_index: usize,
    #[cfg(feature = "debugger")]
    debugger: Debugger,
    debug_enabled: bool,
    /// Timeout information for any operation (test, query, etc.)
    timeout_info: Option<(std::time::Instant, u64)>,
    /// Optional program for accessing type information during execution
    program: Option<Rc<crate::interpreter::compiler::ir::Program>>,
}

impl Solver {
    pub fn new(context: <DefaultUser as User>::UserContext, debug_enabled: bool) -> Solver {
        let engine = DefaultEngine::new();
        #[cfg(feature = "debugger")]
        let debugger = Debugger::new();
        Solver {
            context,
            engine,
            stream_iter_index: 0,
            #[cfg(feature = "debugger")]
            debugger,
            debug_enabled,
            timeout_info: None,
            program: None,
        }
    }

    /// Create a new Solver with program access for type information
    pub fn with_program(
        context: <DefaultUser as User>::UserContext,
        debug_enabled: bool,
        program: Rc<crate::interpreter::compiler::ir::Program>,
    ) -> Solver {
        let engine = DefaultEngine::new();
        #[cfg(feature = "debugger")]
        let debugger = Debugger::new();
        Solver {
            context,
            engine,
            stream_iter_index: 0,
            #[cfg(feature = "debugger")]
            debugger,
            debug_enabled,
            timeout_info: None,
            program: Some(program),
        }
    }

    /// Set timeout information for any operation (test, query, etc.)
    pub fn set_timeout(&mut self, start_time: std::time::Instant, timeout_ms: u64) {
        self.timeout_info = Some((start_time, timeout_ms));
    }

    /// Clear timeout information
    pub fn clear_timeout(&mut self) {
        self.timeout_info = None;
    }

    pub fn start(&self, goal: &Goal, state: State) -> Stream {
        match goal {
            Goal::Succeed => Stream::unit(Box::new(state)),
            Goal::Fail => Stream::empty(),
            Goal::Breakpoint(_id) => {
                if self.debug_enabled {
                    // TODO: self.debugger.breakpoint(goal, &state, *id)
                }
                Stream::unit(Box::new(state))
            }
            Goal::Dynamic(dynamic) => dynamic.solve(self, state),
            Goal::LazyMacro(closure) => closure.expand_and_solve(self, state),
        }
    }

    pub fn start_dfs(&self, goal: &DFSGoal, state: State) -> Stream {
        match goal {
            DFSGoal::Succeed => Stream::unit(Box::new(state)),
            DFSGoal::Fail => Stream::empty(),
            DFSGoal::Breakpoint(_id) => {
                if self.debug_enabled {
                    // TODO: self.debugger.breakpoint(goal, &state, *id)
                }
                Stream::unit(Box::new(state))
            }
            DFSGoal::Dynamic(dynamic) => {
                if self.debug_enabled {
                    // TODO: self.debugger.start(goal, &state)
                }
                dynamic.solve(self, state)
            }
            DFSGoal::LazyMacro(closure) => closure.expand_and_solve(self, state),
        }
    }

    pub fn next(&mut self, stream: &mut Stream) -> SolverResult {
        loop {
            // Increment iteration counter
            self.stream_iter_index += 1;

            // Check timeout periodically to reduce overhead (every 100 iterations)
            // Always check on first iteration for responsiveness
            if self.stream_iter_index == 1 || self.stream_iter_index % TIMEOUT_CHECK_FREQUENCY == 0
            {
                // Check if any timeout has been exceeded
                if let Some((start_time, timeout_ms)) = self.timeout_info {
                    let elapsed = start_time.elapsed().as_millis();
                    if elapsed >= timeout_ms as u128 {
                        return SolverResult::Timeout;
                    }
                }
            }

            #[cfg(feature = "debugger")]
            if self.debug_enabled {
                self.debugger.next_step(stream);
            }
            match std::mem::replace(stream, Stream::Empty) {
                Stream::Empty => {
                    #[cfg(feature = "debugger")]
                    if self.debug_enabled {
                        self.debugger.program_exit();
                    }
                    return SolverResult::NoMoreSolutions;
                }
                Stream::Unit(state) => {
                    #[cfg(feature = "debugger")]
                    if self.debug_enabled {
                        self.debugger.new_solution(stream, &state);
                    }
                    return SolverResult::Solution(state);
                }
                Stream::Lazy(LazyStream(lazy)) => *stream = self.engine.step(self, *lazy),
                Stream::Cons(state, lazy_stream) => {
                    *stream = Stream::Lazy(lazy_stream);
                    #[cfg(feature = "debugger")]
                    if self.debug_enabled {
                        self.debugger.new_solution(stream, &state);
                    }
                    return SolverResult::Solution(state);
                }
                Stream::Error(msg) => {
                    return SolverResult::Error(msg);
                }
            }
        }
    }

    /// Returns a reference to next element in the stream, if any.
    pub fn peek<'a>(&self, stream: &'a mut Stream) -> Option<&'a Box<State>> {
        loop {
            match stream {
                Stream::Lazy(_) => {
                    if let Stream::Lazy(LazyStream(lazy)) = std::mem::replace(stream, Stream::Empty)
                    {
                        *stream = self.engine.step(self, *lazy);
                    }
                }
                _ => return stream.head(),
            }
        }
    }

    /// Truncates the stream leaving at most one element, and returns a reference to
    /// the remaining element if any.
    pub fn trunc<'a>(&self, stream: &'a mut Stream) -> Option<&'a Box<State>> {
        loop {
            match std::mem::replace(stream, Stream::Empty) {
                Stream::Empty => return None,
                Stream::Lazy(LazyStream(lazy)) => {
                    *stream = self.engine.step(self, *lazy);
                }
                Stream::Unit(a) | Stream::Cons(a, _) => {
                    *stream = Stream::Unit(a);
                    return stream.head();
                }
                Stream::Error(_) => {
                    return None; // Error terminates the stream
                }
            }
        }
    }

    pub fn context(&self) -> &<DefaultUser as User>::UserContext {
        &self.context
    }

    pub fn engine(&self) -> &DefaultEngine {
        &self.engine
    }

    pub fn program(&self) -> Option<&Rc<crate::interpreter::compiler::ir::Program>> {
        self.program.as_ref()
    }

    /// Set the IR program for accessing type information during execution
    pub fn set_program(&mut self, program: Rc<crate::interpreter::compiler::ir::Program>) {
        self.program = Some(program);
    }
}

pub trait Solve: fmt::Debug + AnySolve {
    /// Generate a stream of solutions to the goal by applying it to some initial state.
    fn solve(&self, solver: &Solver, state: State) -> Stream;
}

pub trait AnySolve: Any {
    fn as_any(&self) -> &dyn Any;
}

impl<T> AnySolve for T
where
    T: Solve,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl dyn Solve {
    #[inline]
    pub fn is<T: Solve>(&self) -> bool {
        TypeId::of::<T>() == self.type_id()
    }

    #[inline]
    pub fn downcast_ref<T: Any + Solve>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }
}
