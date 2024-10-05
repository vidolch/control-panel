mod models;
use crossbeam_channel::{unbounded, Receiver, Sender};
use models::*;

mod ui;
use ui::*;

mod worker;
use worker::*;

use std::{
    env,
    io::{self, stdout},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        event::DisableMouseCapture,
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
        ExecutableCommand,
    },
    Terminal,
};

use std::fs;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let config_file = if args.len() > 1 {
        args[1].clone()
    } else {
        "./config.yml".to_string()
    };

    let app = Arc::new(Mutex::new(App::default()));

    let contents = fs::read_to_string(config_file).unwrap();

    let cfg: Cfg = serde_yaml::from_str(&contents.to_owned()).unwrap();

    for r_group in cfg.groups.iter() {
        let mut group: Vec<usize> = vec![];
        for r_cfg in r_group.runners.iter() {
            let (r_tx, r_rx): (Sender<RunnerEvent>, Receiver<RunnerEvent>) = unbounded();
            group.push(app.lock().unwrap().runners.len());
            app.lock().unwrap().runners.push(Runner {
                name: r_cfg.name.to_owned(),
                args: r_cfg.args.clone(),
                lines: Vec::new(),
                vertical_scroll_size: 0,
                vertical_scroll_position: 0,
                // horizontal_scroll_size: 0,
                // horizontal_scroll_state: ScrollbarState::new(0).position(0),
                state: RunnerState::Ready,
                should_restart: false,
                tx: r_tx,
                rx: r_rx,
            });
        }
        app.lock().unwrap().groups.push(Group { runners: group });
    }

    let mut ui_app = app.clone();
    let ui_process = thread::spawn(move || {
        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

        let tick_rate = Duration::from_millis(250);
        //let mut u2 = ui_app.lock().unwrap();
        let res = run_app(&mut terminal, &mut ui_app, tick_rate);

        // restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        res
    });

    let worker_app = app.clone();
    let woker_process = thread::spawn(move || {
        // Start the process

        let mut handlers = vec![];

        let mut index = 0;
        for runner_group in cfg.groups {
            for runner_config in runner_group.runners {
                let runner_app = worker_app.clone();
                let runner_index = index.clone();
                let handle =
                    thread::spawn(move || start_worker(runner_app, runner_config, runner_index));

                handlers.push(handle);
                index += 1;
            }
        }

        for handle in handlers {
            handle.join().expect("Handler paniced");
        }
    });

    if let Err(err) = ui_process.join() {
        println!("{err:?}");
    }

    if let Err(err) = woker_process.join() {
        println!("{err:?}");
    }

    Ok(())
}
