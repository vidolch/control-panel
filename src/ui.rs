use crate::models::*;

use std::{
    io, iter,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
    vec,
};

use ratatui::{
    crossterm::event::{self, Event, KeyCode},
    layout::{Constraint, Flex, Layout, Margin, Rect},
    prelude::Backend,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame, Terminal,
};

pub fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: &mut Arc<Mutex<App>>,
    tick_rate: Duration,
) -> Result<(), B::Error>
where
    B::Error: From<io::Error>,
{
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                let mut app = app.lock().unwrap();
                match key.code {
                    KeyCode::Char('q') => {
                        app.should_exit = true;
                        for runner in app.runners.iter() {
                            let _ = runner.tx.send(RunnerEvent {
                                event_type: EventType::ApplicationQuit,
                            });
                        }
                        return Ok(());
                    }
                    KeyCode::Char('r') => {
                        let active_runner = app.active_runner;
                        let active_runner = app.runners.get_mut(active_runner).unwrap();
                        active_runner.should_restart = true;

                        let _ = active_runner.tx.send(RunnerEvent {
                            event_type: EventType::Restart,
                        });
                    }
                    KeyCode::Char('s') => {
                        let active_runner = app.active_runner;
                        let active_runner = app.runners.get_mut(active_runner).unwrap();
                        active_runner.should_restart = true;

                        let _ = active_runner.tx.send(RunnerEvent {
                            event_type: EventType::Stop,
                        });
                    }
                    KeyCode::Char('n') => {
                        if app.active_runner == app.runners.len() - 1 {
                            app.active_runner = 0;
                        } else {
                            app.active_runner += 1;
                        }
                    }
                    KeyCode::Char('p') => {
                        if app.active_runner == 0 {
                            app.active_runner = app.runners.len() - 1;
                        } else {
                            app.active_runner -= 1;
                        }
                    }
                    KeyCode::Char('t') => {
                        app.show_timestamps = !app.show_timestamps;
                    }
                    KeyCode::Char('z') => {
                        if app.has_zoomed_runner {
                            app.has_zoomed_runner = false;
                        } else {
                            app.has_zoomed_runner = true;
                            app.zoomed_runner = app.active_runner;
                        }
                    }
                    KeyCode::Char('d') => {
                        app.show_debug = !app.show_debug;
                    }
                    KeyCode::Char('?') => {
                        app.show_help = true;
                    }
                    KeyCode::Esc => {
                        app.show_help = false;
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        let active_runner = app.active_runner;
                        let active_runner = app.runners.get_mut(active_runner).unwrap();
                        active_runner.vertical_scroll_position = if active_runner
                            .vertical_scroll_position
                            < active_runner.vertical_scroll_size
                        {
                            active_runner.vertical_scroll_position.saturating_add(1)
                        } else {
                            active_runner.vertical_scroll_position
                        };
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        let active_runner = app.active_runner;
                        let active_runner = app.runners.get_mut(active_runner).unwrap();
                        active_runner.vertical_scroll_position =
                            if active_runner.vertical_scroll_position > 0 {
                                active_runner.vertical_scroll_position.saturating_sub(1)
                            } else {
                                active_runner.vertical_scroll_position
                            };
                    }
                    _ => (),
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

pub fn ui(frame: &mut Frame, app: &mut Arc<Mutex<App>>) {
    let app = app.lock().unwrap();

    let [main_area_width, debug_area_width] = if app.show_debug { [80, 20] } else { [100, 0] };
    let [main_area, debug_area] = Layout::horizontal(vec![
        Constraint::Percentage(main_area_width),
        Constraint::Percentage(debug_area_width),
    ])
    .areas(frame.area());

    let mut constraints = vec![Constraint::Length(1)];
    if app.has_zoomed_runner {
        constraints.push(Constraint::Min(0));
    } else {
        constraints.extend(app.groups.iter().map(|_g| Constraint::Min(0)).into_iter());
    }
    constraints.push(Constraint::Length(1));

    let left_areas = Layout::vertical(constraints).split(main_area);

    let title_area = left_areas[0];
    frame.render_widget(Block::bordered().title("Control panel"), title_area);

    let status_area = left_areas[left_areas.len() - 1];
    frame.render_widget(
        Block::bordered().title("Use <N> to scroll panes, <R> to restart process, <?> for all key-bindings, <Q> to exit")
        ,status_area
    );

    let main_areas = &left_areas[1..left_areas.len() - 1];

    let mut area_group_index = 0;
    let areas: Vec<_> = main_areas
        .iter()
        .map(|main_area| {
            let inner_areas = if app.has_zoomed_runner {
                1
            } else {
                app.groups[area_group_index].runners.len()
            };

            let a = Layout::horizontal(
                iter::repeat(Constraint::Percentage(100))
                    .take(inner_areas)
                    .collect::<Vec<Constraint>>(),
            )
            .split(main_area.to_owned());
            area_group_index += 1;
            a
        })
        .collect();

    if app.has_zoomed_runner {
        let row_area = &areas[0];
        let area = row_area[0];
        render_runner_pane(&app, app.zoomed_runner, area, frame)
    } else {
        for (group_index, group) in app.groups.iter().enumerate() {
            let row_area = &areas[group_index];

            for (runner_iter_index, runner_index) in group.runners.iter().enumerate() {
                let area = row_area[runner_iter_index];
                render_runner_pane(&app, *runner_index, area, frame);
            }
        }
    }

    if app.show_help {
        let popup_block = Paragraph::new(vec![
            Line::from("<N> - Next pane"),
            Line::from("<P> - Previous pane"),
            Line::from("<Z> - Zoom out/in pane"),
            Line::from("<T> - Toggle timestamps"),
            Line::from("<S> - Stop process in the active pane"),
            Line::from("<R> - Restart process in the active pane"),
            Line::from("<Q> - Quit app"),
            Line::from("<D> - Toggle debug window"),
        ])
        .block(
            Block::bordered()
                .title("Keybindings")
                .border_style(Style::new().green())
                .style(Style::default().bg(Color::Black)),
        );

        let area = popup_area(frame.area(), 60, 60);
        frame.render_widget(Clear, area);
        frame.render_widget(popup_block, area)
    }

    if app.show_debug {
        frame.render_widget(
            Paragraph::new(
                app.debug_lines
                    .clone()
                    .iter()
                    .map(|l| l.to_ratatui_line(false))
                    .collect::<Vec<Line>>(),
            )
            .block(
                Block::bordered()
                    .title("Debug Logs")
                    .border_style(Style::new().green())
                    .style(Style::default().bg(Color::Black)),
            ),
            debug_area,
        )
    }
}

fn render_runner_pane(app: &MutexGuard<App>, runner_index: usize, area: Rect, frame: &mut Frame) {
    let r = &app.runners[runner_index];

    let mut title = r.name.clone();
    if app.active_runner == runner_index {
        title = " > ".to_owned() + &title;
    }

    let style = match r.state {
        RunnerState::Ready => Style::new().blue(),
        RunnerState::Error => Style::new().red(),
        RunnerState::Active => Style::new().gray(),
        RunnerState::Finish => Style::new().green(),
    };

    let mut scroll: usize = r.vertical_scroll_position.try_into().unwrap();
    let mut scroll_size: usize = r.vertical_scroll_size.try_into().unwrap();
    let height: usize = area.height.try_into().unwrap();
    scroll = scroll.saturating_sub(height);
    scroll_size = scroll_size.saturating_sub(height);

    let mut lines: Vec<Line> = Vec::new();
    let empty_height = r.vertical_scroll_position.saturating_sub(height);
    lines.append(&mut vec![Line::from(""); empty_height]);
    lines.append(
        &mut r.lines[empty_height..r.vertical_scroll_position]
            .iter()
            .map(|x| x.to_ratatui_line(app.show_timestamps))
            .collect::<Vec<Line>>(),
    );
    lines.append(&mut vec![
        Line::from("");
        r.vertical_scroll_size.saturating_sub(empty_height)
    ]);

    frame.render_widget(
        Paragraph::new(
            lines, // r.lines
                  //     .clone()
                  //     .iter()
                  //     .map(|l| l.to_ratatui_line(app.show_timestamps))
                  //     .collect::<Vec<Line>>(),
        )
        .scroll((scroll.try_into().unwrap(), 0))
        .block(Block::bordered().border_style(style).title(title)),
        area,
    );

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    let mut scrollbar_state = ScrollbarState::new(scroll_size).position(scroll);

    frame.render_stateful_widget(
        scrollbar,
        area.inner(Margin {
            // using an inner vertical margin of 1 unit makes the scrollbar inside the block
            vertical: 1,
            horizontal: 0,
        }),
        &mut scrollbar_state,
    );
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
