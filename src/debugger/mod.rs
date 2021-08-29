use crate::engine::Engine;
use crate::goal::Goal;
use crate::solver::Solver;
use crate::state::State;
use crate::stream::{Lazy, Stream};
use crate::user::User;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use tui::{backend::CrosstermBackend, Terminal};

mod ui;
use ui::UI;

pub struct Model<U, E>
where
    U: User,
    E: Engine<U>,
{
    has_quit: bool,
    stream: Stream<U, E>,
}

impl<U, E> Model<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new() -> Model<U, E> {
        Model {
            has_quit: false,
            stream: Stream::Empty,
        }
    }
}

pub struct Debugger<U, E>
where
    U: User,
    E: Engine<U>,
{
    ui: UI<U, E>,
    model: Model<U, E>,
}

impl<U, E> Debugger<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new() -> Debugger<U, E> {
        let ui = UI::new();
        let model = Model::new();
        Debugger { ui, model }
    }

    pub fn process_events(&mut self) {}

    pub fn next_step(&mut self, stream: &Stream<U, E>) {
        if self.model.has_quit {
            return;
        }

        // Update debugger data model with new stream
        self.model.stream = stream.clone();

        // Refresh view
        self.ui.show();
        self.ui.main(&mut self.model);

        // if continue, hide UI, if just step, do not hide UI
    }

    // Stream became empty, no more solutions => program exit
    pub fn program_exit(&mut self) {
        self.ui.hide();
    }

    pub fn new_solution(&mut self, stream: &Stream<U, E>, state: &Box<State<U, E>>) {}
}
