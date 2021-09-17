use crate::engine::Engine;
use crate::stream::{Lazy, StreamWalkStep};
use crate::user::User;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use tui::{
    backend::CrosstermBackend,
    layout::Constraint,
    layout::Direction,
    layout::Layout,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem},
    Frame, Terminal,
};

use super::Model;

enum Event {
    Input(KeyEvent),
    Tick,
}

pub struct UI<U, E>
where
    U: User,
    E: Engine<U>,
{
    io_thread_handle: thread::JoinHandle<()>,
    event_receiver: mpsc::Receiver<Event>,
    ui_visible: Arc<AtomicBool>,

    terminal: Terminal<CrosstermBackend<std::io::Stdout>>,

    _phantom: PhantomData<U>,
    _phantom2: PhantomData<E>,
}

impl<U, E> UI<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new() -> UI<U, E> {
        let (event_sender, event_receiver) = mpsc::channel();
        let tick_rate = Duration::from_millis(200);
        let ui_visible = Arc::new(AtomicBool::new(false));
        let io_ui_visible = Arc::clone(&ui_visible);

        // Spawn IO polling thread that sends IO and/or timetick events to
        // the channel if the UI is visible.
        let io_thread_handle = thread::spawn(move || {
            let mut last_tick = Instant::now();
            loop {
                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or_else(|| Duration::from_secs(0));
                let ui_visible = io_ui_visible.load(Ordering::SeqCst);

                if event::poll(timeout).expect("poll works") {
                    if let CEvent::Key(key) = event::read().expect("can read events") {
                        // Generate key events to queue only if the UI is visible
                        if ui_visible {
                            event_sender
                                .send(Event::Input(key))
                                .expect("can send events");
                        }
                    }
                }
                if last_tick.elapsed() >= tick_rate {
                    // Generate tick events to queue only if the UI is visible
                    if ui_visible {
                        event_sender.send(Event::Tick).unwrap();
                    }
                    last_tick = Instant::now();
                }
            }
        });

        let stdout = std::io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend).expect("new terminal");

        UI {
            io_thread_handle,
            event_receiver,
            ui_visible,
            terminal,
            _phantom: PhantomData,
            _phantom2: PhantomData,
        }
    }

    pub fn show(&mut self) {
        if self.ui_visible.load(Ordering::SeqCst) {
            return;
        }

        // Enable event generation
        self.ui_visible.store(true, Ordering::SeqCst);

        enable_raw_mode().expect("can run in raw mode");
        execute!(
            self.terminal.backend_mut(),
            EnterAlternateScreen,
            EnableMouseCapture
        )
        .expect("success");
        self.terminal.hide_cursor().expect("hidden cursor");
        self.terminal.clear().expect("cleared terminal");
    }

    pub fn hide(&mut self) {
        if !self.ui_visible.load(Ordering::SeqCst) {
            return;
        }

        // Disable event generation
        self.ui_visible.store(false, Ordering::SeqCst);

        disable_raw_mode().expect("can disable raw mode");
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )
        .expect("success");
        self.terminal.show_cursor().expect("visible cursor");
    }

    fn draw(frame: &mut Frame<CrosstermBackend<std::io::Stdout>>, model: &Model<U, E>) {
        let size = frame.size();
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .margin(0)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(size);
        let block = Block::default().title("Stream").borders(Borders::ALL);
        let block2 = Block::default().title("Substitution").borders(Borders::ALL);
        let mut stream_items: Vec<ListItem> = vec![];
        let mut stream_walker = model.stream.walk();
        loop {
            match stream_walker.next() {
                Some((depth, step)) => {
                    let indent = " ".repeat(depth);
                    match step {
                        StreamWalkStep::State(_state) => {
                            let item = format!("{}{}", indent, "State");
                            stream_items.push(ListItem::new(item));
                        }
                        StreamWalkStep::LazyStream(lazy_stream) => {
                            let item = match &*lazy_stream.0 {
                                Lazy::Bind(_lazy_stream, goal) => {
                                    format!("{}Bind: {:?}", indent, goal)
                                }
                                Lazy::MPlus(_left, _right) => {
                                    format!("{}{}", indent, "MPlus")
                                }
                                Lazy::Pause(_state, goal) => {
                                    format!("{}Pause: {:?}", indent, goal)
                                }
                                Lazy::BindDFS(_lazy_stream, goal) => {
                                    format!("{}Bind: {:?}", indent, goal)
                                }
                                Lazy::MPlusDFS(_left, _right) => {
                                    format!("{}{}", indent, "MPlusDFS")
                                }
                                Lazy::PauseDFS(_state, goal) => {
                                    format!("{}PauseDFS: {:?}", indent, goal)
                                }
                                Lazy::Delay(_stream) => {
                                    format!("{}{}", indent, "Delay")
                                }
                                Lazy::Iterator(_iter) => {
                                    format!("{}{}", indent, "Iterator")
                                }
                            };
                            stream_items.push(ListItem::new(item));
                        }
                        StreamWalkStep::Backtrack(_lazy_stream) => {}
                    };
                }
                None => break,
            }
        }

        let mut smap_items = vec![];
        if !model.stream.is_empty() && model.stream.is_mature() {
            let head = model.stream.head().unwrap();
            let smap = head.smap_ref();
            for (key, value) in smap.iter() {
                assert!(key.is_var());
                let name = key.get_name().unwrap();
                let walked_value = smap.walk(value);
                let item = format!("{}: {:?}", name, walked_value);
                smap_items.push(ListItem::new(item));
            }
        } else {
            smap_items.push(ListItem::new("Immature stream"));
        }

        let list = List::new(stream_items)
            .block(block)
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
            .highlight_symbol(">>");

        let list2 = List::new(smap_items)
            .block(block2)
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
            .highlight_symbol(">>");

        frame.render_widget(list, chunks[0]);
        frame.render_widget(list2, chunks[1]);
    }

    pub fn main(&mut self, model: &mut Model<U, E>) {
        loop {
            let terminal = &mut self.terminal;
            terminal.draw(|f| UI::draw(f, model)).expect("something");

            match self.event_receiver.recv().expect("success") {
                Event::Input(event) => match event.code {
                    KeyCode::Char('q') => {
                        self.hide();
                        model.has_quit = true;
                        break;
                    }
                    KeyCode::Char('s') => {
                        break;
                    }
                    _ => (),
                },
                Event::Tick => {
                    // app.on_tick()
                }
            }
        }
    }
}
