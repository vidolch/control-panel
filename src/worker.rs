use crate::{App, EventType, RunnerCfg, RunnerEvent, RunnerState, StdLine};
use crossbeam_channel::{Receiver, Sender};
use std::{
    io::{self, BufRead},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
    usize,
};

pub fn start_worker(app: Arc<Mutex<App>>, runner_config: RunnerCfg, runner_index: usize) {
    start_process(app, runner_config, runner_index)
}

fn start_process(app: Arc<Mutex<App>>, runner_config: RunnerCfg, runner_index: usize) {
    let command_handle = Arc::new(Mutex::new(None::<Child>));
    let child_join = Arc::new(Mutex::new(None::<JoinHandle<()>>));
    #[allow(unused_assignments)]
    let mut rx = None::<Receiver<RunnerEvent>>;
    #[allow(unused_assignments)]
    let mut tx = None::<Sender<RunnerEvent>>;

    {
        let process_app = app.lock().unwrap();
        rx = Some(process_app.runners[runner_index].rx.clone());
        tx = Some(process_app.runners[runner_index].tx.clone());
    }
    if runner_config.auto_start {
        let mut handle = command_handle.lock().unwrap();
        let (child, join) = spawn_child(
            app.clone(),
            runner_config.clone(),
            runner_index,
            tx.clone().unwrap(),
        );
        *handle = Some(child);
        let mut cjoin = child_join.lock().unwrap();
        *cjoin = Some(join);
    }

    'process_loop: loop {
        let mut handle = command_handle.lock().unwrap();
        if let Ok(event) = rx.clone().unwrap().recv() {
            match event.event_type {
                EventType::Stop => {
                    if handle.is_some() {
                        let _ = handle.as_mut().unwrap().kill();
                    }
                }
                EventType::Restart => {
                    if handle.is_some() {
                        let _ = handle.as_mut().unwrap().kill();
                    }

                    let (child, join) = spawn_child(
                        app.clone(),
                        runner_config.clone(),
                        runner_index,
                        tx.clone().unwrap(),
                    );
                    *handle = Some(child);
                    let mut cjoin = child_join.lock().unwrap();
                    *cjoin = Some(join);
                }
                EventType::Finish => {
                    let mut join = child_join.lock().unwrap();
                    if let Some(ref inner_join) = *join {
                        if inner_join.is_finished() {
                            let mut process_app = app.lock().unwrap();
                            let jinner_join = join.take().unwrap();
                            let res = jinner_join.join().is_ok();
                            if res {
                                process_app.runners[runner_index].state = RunnerState::Finish;
                            } else {
                                process_app.runners[runner_index].state = RunnerState::Error;
                            }
                        }
                    }

                    if runner_config.restart_on_finish {
                        let _ = tx.clone().unwrap().send(RunnerEvent {
                            event_type: EventType::Restart,
                        });
                    }
                }
                EventType::ApplicationQuit => {
                    if handle.is_some() {
                        let _ = handle.as_mut().unwrap().kill();
                    }
                    break 'process_loop;
                }
            }
        }
    }
}

fn spawn_child(
    app: Arc<Mutex<App>>,
    runner_config: RunnerCfg,
    runner_index: usize,
    tx: Sender<RunnerEvent>,
) -> (Child, JoinHandle<()>) {
    let lead = runner_config.args[0].to_owned();
    let args: Vec<&String> = runner_config.args.iter().skip(1).collect();

    let mut cmd = &mut Command::new(&lead);
    for arg in args {
        cmd = cmd.arg(arg);
    }
    if runner_config.dir != "" {
        cmd = cmd.current_dir(runner_config.dir)
    }
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start ping process");

    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let stdout_reader = io::BufReader::new(stdout);

    let stderr = child.stderr.take().expect("Failed to capture stdout");
    let stderr_reader = io::BufReader::new(stderr);

    let reader_app = app.clone();
    let join = thread::spawn(move || {
        let out_reader = reader_app.clone();
        let stdout_join = thread::spawn(move || {
            // read the lines 1 by 1
            for line in stdout_reader.lines() {
                let mut process_app = out_reader.lock().unwrap();
                let runner = &mut process_app.runners.get_mut(runner_index).unwrap();

                let s = line.expect("Could not get a line");
                runner.lines.push(StdLine::new(s));
                if runner.vertical_scroll_position == runner.vertical_scroll_size {
                    runner.vertical_scroll_position =
                        runner.vertical_scroll_position.saturating_add(1);
                }
                runner.vertical_scroll_size = runner.vertical_scroll_size.saturating_add(1);
            }
        });

        let err_reader = reader_app.clone();
        let stderr_join = thread::spawn(move || {
            // read the lines 1 by 1
            for line in stderr_reader.lines() {
                let mut process_app = err_reader.lock().unwrap();
                let runner = &mut process_app.runners.get_mut(runner_index).unwrap();

                let s = line.expect("Could not get a line");
                runner.lines.push(StdLine::new(s));
                if runner.vertical_scroll_position == runner.vertical_scroll_size {
                    runner.vertical_scroll_position =
                        runner.vertical_scroll_position.saturating_add(1);
                }
                runner.vertical_scroll_size = runner.vertical_scroll_size.saturating_add(1);
            }
        });

        let _ = stdout_join.join();
        let _ = stderr_join.join();
        let _ = tx.send(RunnerEvent {
            event_type: EventType::Finish,
        });
    });

    {
        let mut process_app = app.lock().unwrap();

        process_app.runners[runner_index].state = RunnerState::Active;
        process_app.runners[runner_index].should_restart = false;
    }

    (child, join)
}
