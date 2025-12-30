mod data;
mod model;

use std::{collections::HashMap, error::Error, io, time::Duration};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use data::DataClient;
use image::{DynamicImage, GenericImageView};
use model::Event as GameEvent;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols::Marker,
    text::{Line, Span},
    widgets::{canvas::{Canvas, Points}, Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use tokio::sync::mpsc;
use tui_big_text::{BigText, PixelSize};

#[derive(Debug)]
struct App {
    should_quit: bool,
    events: Vec<GameEvent>,
    state: ListState,
    filter_live: bool,
    logos: HashMap<String, DynamicImage>,
    show_logos: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            should_quit: false,
            events: Vec::new(),
            state: ListState::default(),
            filter_live: false,
            logos: HashMap::new(),
            show_logos: true,
        }
    }
}

impl App {
    fn new() -> Self {
        let mut app = Self::default();
        app.state.select(Some(0));
        app
    }

    fn on_key(&mut self, c: char) {
        match c {
            'q' => self.should_quit = true,
            'j' => self.next(),
            'k' => self.previous(),
            'l' => self.toggle_live_filter(),
            _ => {}
        }
    }

    fn next(&mut self) {
        if self.filtered_events().is_empty() { return; }
        
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.filtered_events().len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        if self.filtered_events().is_empty() { return; }

        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.filtered_events().len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn toggle_live_filter(&mut self) {
        self.filter_live = !self.filter_live;
        self.state.select(Some(0));
    }

    fn filtered_events(&self) -> Vec<&GameEvent> {
        if !self.filter_live {
            self.events.iter().collect()
        } else {
            self.events
                .iter()
                .filter(|e| e.status.type_field.state == "in")
                .collect()
        }
    }
}

fn parse_color(hex: &str) -> Color {
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        Color::Rgb(r, g, b)
    } else {
        Color::White
    }
}

use clap::Parser;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Update interval in seconds
    #[arg(short, long, default_value_t = 15)]
    interval: u64,

    /// Use NCAA College Football instead of NFL
    #[arg(long)]
    ncaa: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new();

    // Setup channel for background updates
    let (tx, mut rx) = mpsc::channel::<(Vec<GameEvent>, Option<(String, DynamicImage)>)>(100);

    // Initial fetch
    let client = Arc::new(crate::data::DataClient::new());
    let client_clone = client.clone();
    let tx_clone = tx.clone();
    let interval_secs = args.interval;
    let league = if args.ncaa { "college-football" } else { "nfl" }.to_string();

    // Spawn background data fetching task
    tokio::spawn(async move {
        let mut fetched_logos: std::collections::HashSet<String> = std::collections::HashSet::new();

        loop {
            if let Ok(data) = client_clone.fetch_scoreboard(&league).await {
                // Check for logos
                for event in &data.events {
                    for comp in &event.competitions {
                        for competitor in &comp.competitors {
                           let abbr = &competitor.team.abbreviation;
                           if !fetched_logos.contains(abbr) {
                               if let Some(url) = &competitor.team.logo {
                                   if let Ok(resp) = reqwest::get(url).await {
                                       if let Ok(bytes) = resp.bytes().await {
                                            if let Ok(img) = image::load_from_memory(&bytes) {
                                                 let _ = tx_clone.send((Vec::new(), Some((abbr.clone(), img)))).await;
                                                 fetched_logos.insert(abbr.clone());
                                            }
                                       }
                                   }
                               }
                           }
                        }
                    }
                }
                
                let _ = tx_clone.send((data.events, None)).await;
            }
            tokio::time::sleep(Duration::from_secs(interval_secs)).await;
        }
    });

    // Run app loop
    let res = run_app(&mut terminal, &mut app, &mut rx).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    rx: &mut mpsc::Receiver<(Vec<GameEvent>, Option<(String, DynamicImage)>)>,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => app.should_quit = true,
                    KeyCode::Char('l') => app.show_logos = !app.show_logos,
                    KeyCode::Char('f') => app.toggle_live_filter(), // 'f' for filter, as 'l' is now for logos
                    KeyCode::Down | KeyCode::Char('j') => app.next(),
                    KeyCode::Up | KeyCode::Char('k') => app.previous(),
                    _ => {}
                }
            }
        }

        while let Ok((events, logo_update)) = rx.try_recv() {
            if !events.is_empty() {
                app.events = events;
            }
            if let Some((abbr, img)) = logo_update {
                app.logos.insert(abbr, img);
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let size = f.area();
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(25),
                Constraint::Percentage(75),
            ]
            .as_ref(),
        )
        .split(size);

    draw_sidebar(f, app, chunks[0]);
    draw_main_panel(f, app, chunks[1]);
}

fn draw_sidebar(f: &mut Frame, app: &mut App, area: Rect) {
    let events = app.filtered_events();
    let items: Vec<ListItem> = events
        .iter()
        .map(|game| {
            let title = &game.short_name;
            let status = if game.status.type_field.state == "pre" {
                 "Pre".to_string()
            } else if game.status.type_field.state == "post" {
                 "Final".to_string()
            } else {
                 game.status.display_clock.clone()
            };
            
            let content = format!("{}  [{}]", title, status);
            ListItem::new(content)
        })
        .collect();

    let title = if app.filter_live { " LIVE GAMES " } else { " GAMES " };

    let games_list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).bg(Color::DarkGray).fg(Color::White));

    f.render_stateful_widget(games_list, area, &mut app.state);
}

fn draw_main_panel(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default().borders(Borders::ALL);
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let events = app.filtered_events();
    let selected_index = app.state.selected().unwrap_or(0);

    if let Some(game) = events.get(selected_index) {
        if let Some(comp) = game.competitions.first() {
            let home = comp.competitors.iter().find(|c| c.home_away == "home");
            let away = comp.competitors.iter().find(|c| c.home_away == "away");

            if let (Some(h), Some(a)) = (home, away) {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(16), // Scoreboard Header
                        Constraint::Min(6),     // Field Display (Allow shrinking)
                        Constraint::Length(1),  // Spacer
                        Constraint::Length(3),  // Status Bar
                        Constraint::Min(0),     // Details
                    ].as_ref())
                    .split(inner_area);

                // --- Scoreboard Header ---
                let header_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(40), // Away
                        Constraint::Percentage(20), // VS/Clock
                        Constraint::Percentage(40), // Home
                    ].as_ref())
                    .split(chunks[0]);

                // Away Team
                let a_color = parse_color(a.team.color.as_deref().unwrap_or("000000"));
                let a_block = Block::default().bg(a_color);
                f.render_widget(a_block, header_chunks[0]);

                let a_constraints = if app.show_logos {
                     [Constraint::Length(22), Constraint::Min(0)]
                } else {
                     [Constraint::Length(0), Constraint::Min(0)]
                };

                let a_content_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(a_constraints.as_ref()) 
                    .split(header_chunks[0]);

                  if app.show_logos {
                      if let Some(img) = app.logos.get(&a.team.abbreviation) {
                         let canvas = Canvas::default()
                            .block(Block::default())
                            .marker(Marker::Braille)
                            .x_bounds([0.0, 40.0])
                            .y_bounds([0.0, 20.0])
                            .paint(|ctx| {
                                let (w, h) = img.dimensions();
                                for y in 0..40 {
                                    for x in 0..80 {
                                        let img_x = (x as f64 / 80.0 * w as f64) as u32;
                                        let img_y = (y as f64 / 40.0 * h as f64) as u32;
                                        if img_x < w && img_y < h {
                                             let p = img.get_pixel(img_x, img_y);
                                             let alpha = p[3];
                                             if alpha > 128 {
                                                  ctx.draw(&Points {
                                                      coords: &[(x as f64 / 2.0, 20.0 - (y as f64 / 2.0))],
                                                      color: Color::Rgb(p[0], p[1], p[2]),
                                                  });
                                             }
                                        }
                                    }
                                }
                            });
                          f.render_widget(canvas, a_content_chunks[0]);
                     }
                  }


                // Stacked Big Text (Abbr + Score)
                let a_text_area = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(1),      // Spacer
                        Constraint::Percentage(40), // Abbr
                        Constraint::Percentage(40), // Score
                        Constraint::Length(1)       // Possession
                    ].as_ref())
                    .split(a_content_chunks[1]);

                if a_text_area[1].width < 25 {
                    let a_abbr_p = Paragraph::new(a.team.abbreviation.clone())
                        .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
                        .alignment(Alignment::Center)
                        .block(Block::default().borders(Borders::NONE)); // No extra borders needed inside
                    f.render_widget(a_abbr_p, a_text_area[1]);
                } else {
                    let a_abbr_text = BigText::builder()
                        .pixel_size(PixelSize::Quadrant)
                        .style(Style::default().fg(Color::White))
                        .lines(vec![format!("  {}  ", a.team.abbreviation.clone()).into()])
                        .alignment(Alignment::Center) 
                        .build();
                    f.render_widget(a_abbr_text, a_text_area[1]);
                }

                let a_score_str = a.score.as_deref().unwrap_or("0").to_string();
                if a_text_area[2].width < 25 {
                     let a_score_p = Paragraph::new(a_score_str)
                        .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
                        .alignment(Alignment::Center);
                    f.render_widget(a_score_p, a_text_area[2]);
                } else {
                    let a_score_text = BigText::builder()
                        .pixel_size(PixelSize::Quadrant)
                        .style(Style::default().fg(Color::White))
                        .lines(vec![a_score_str.into()])
                        .alignment(Alignment::Center)
                        .build();
                    f.render_widget(a_score_text, a_text_area[2]);
                }

                // Possession Indicator
                if let Some(sit) = &comp.situation {
                    if let Some(pos_id) = &sit.possession {
                        if pos_id == &a.team.id.clone().unwrap_or_default() {
                            let p = Paragraph::new("🏈").alignment(Alignment::Center);
                            f.render_widget(p, a_text_area[3]);
                        }
                    }
                }


                // Home Team
                let h_color = parse_color(h.team.color.as_deref().unwrap_or("000000"));
                let h_block = Block::default().bg(h_color);
                f.render_widget(h_block, header_chunks[2]);
                
                let h_constraints = if app.show_logos {
                     [Constraint::Min(0), Constraint::Length(22)]
                } else {
                     [Constraint::Min(0), Constraint::Length(0)]
                };

                 let h_content_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(h_constraints.as_ref()) // Logo on Right
                     .split(header_chunks[2]);

                // Stacked Big Text (Home)
                let h_text_area = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(1),      // Spacer
                        Constraint::Percentage(40), 
                        Constraint::Percentage(40),
                        Constraint::Length(1)       // Possession
                    ].as_ref())
                    .split(h_content_chunks[0]);
                
                if h_text_area[1].width < 25 {
                     let h_abbr_p = Paragraph::new(h.team.abbreviation.clone())
                        .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
                        .alignment(Alignment::Center);
                    f.render_widget(h_abbr_p, h_text_area[1]);
                } else {
                    let h_abbr_text = BigText::builder()
                        .pixel_size(PixelSize::Quadrant)
                        .style(Style::default().fg(Color::White))
                        .lines(vec![format!("  {}  ", h.team.abbreviation.clone()).into()])
                        .alignment(Alignment::Center)
                        .build();
                    f.render_widget(h_abbr_text, h_text_area[1]);
                }

                let h_score_str = h.score.as_deref().unwrap_or("0").to_string();
                if h_text_area[2].width < 25 {
                    let h_score_p = Paragraph::new(h_score_str)
                        .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
                        .alignment(Alignment::Center);
                    f.render_widget(h_score_p, h_text_area[2]);
                } else {
                    let h_big_text = BigText::builder()
                        .pixel_size(PixelSize::Quadrant)
                        .style(Style::default().fg(Color::White))
                        .lines(vec![h_score_str.into()])
                        .alignment(Alignment::Center)
                        .build();
                    f.render_widget(h_big_text, h_text_area[2]);
                }
                
                // Possession Indicator
                if let Some(sit) = &comp.situation {
                    if let Some(pos_id) = &sit.possession {
                        if pos_id == &h.team.id.clone().unwrap_or_default() {
                            let p = Paragraph::new("🏈").alignment(Alignment::Center);
                            f.render_widget(p, h_text_area[3]);
                        }
                    }
                }

                  if app.show_logos {
                      if let Some(img) = app.logos.get(&h.team.abbreviation) {
                         let canvas = Canvas::default()
                            .block(Block::default())
                            .marker(Marker::Braille)
                            .x_bounds([0.0, 40.0])
                            .y_bounds([0.0, 20.0])
                            .paint(|ctx| {
                                let (w, h) = img.dimensions();
                                for y in 0..40 {
                                    for x in 0..80 {
                                        let img_x = (x as f64 / 80.0 * w as f64) as u32;
                                        let img_y = (y as f64 / 40.0 * h as f64) as u32;
                                        if img_x < w && img_y < h {
                                             let p = img.get_pixel(img_x, img_y);
                                             let alpha = p[3];
                                             if alpha > 128 {
                                                  ctx.draw(&Points {
                                                      coords: &[(x as f64 / 2.0, 20.0 - (y as f64 / 2.0))],
                                                      color: Color::Rgb(p[0], p[1], p[2]),
                                                  });
                                             }
                                        }
                                    }
                                }
                            });
                          f.render_widget(canvas, h_content_chunks[1]);
                     }
                  }
                

                // Middle
                let status_color = if game.status.type_field.state == "in" { Color::Red } else { Color::Gray };
                let mid_text = vec![
                    Line::from(""),
                    Line::from(Span::styled("VS", Style::default().add_modifier(Modifier::ITALIC))),
                    Line::from(""),
                    Line::from(Span::styled(&game.status.display_clock, Style::default().fg(status_color).add_modifier(Modifier::BOLD))),
                    Line::from(format!("Q{}", game.status.period)),
                ];
                let mid_p = Paragraph::new(mid_text).alignment(Alignment::Center);
                f.render_widget(mid_p, header_chunks[1]);

                // --- Field Display ---
                let field_display = FieldDisplay {
                    home: h,
                    away: a,
                    comp,
                };
                f.render_widget(field_display, chunks[1]);


                // --- Status Bar ---
                let mut status_line_content = vec![];
                if let Some(sit) = &comp.situation {
                    if let Some(dd_text) = &sit.short_down_distance_text {
                        status_line_content.push(Span::styled(format!(" {} ", dd_text), Style::default().bg(Color::White).fg(Color::Black).add_modifier(Modifier::BOLD)));
                    }
                    if let Some(pos) = &sit.possession {
                         let pos_team = if pos == &a.team.id.clone().unwrap_or_default() {
                             &a.team.abbreviation
                         } else {
                             &h.team.abbreviation
                         };
                         status_line_content.push(Span::raw(format!("  Possession: {}", pos_team)));
                    }
                    if let Some(yl) = sit.yard_line {
                        let yl_text = if yl > 50 {
                            format!("OWN {}", 100 - yl)
                        } else if yl == 50 {
                             "MID".to_string()
                        } else {
                            format!("OPP {}", yl)
                        };
                        status_line_content.push(Span::raw(format!("  at {}", yl_text)));
                    }
                } else {
                    status_line_content.push(Span::raw(format!("  {}", game.status.type_field.detail)));
                }

                // Broadcast Info
                if let Some(broadcasts) = &comp.broadcasts {
                    let names: Vec<String> = broadcasts.iter().flat_map(|b| b.names.clone()).collect();
                    if !names.is_empty() {
                         status_line_content.push(Span::styled(format!("  [TV: {}]", names.join(", ")), Style::default().fg(Color::Cyan)));
                    }
                }
                
                let val_status_line = Line::from(status_line_content);
                let status_bar = Paragraph::new(val_status_line).block(Block::default().borders(Borders::TOP | Borders::BOTTOM));
                f.render_widget(status_bar, chunks[3]);


                // --- Details ---
                if let Some(sit) = &comp.situation {
                    if let Some(lp) = &sit.last_play {
                        let details = vec![
                            Line::from(Span::styled("Last Play", Style::default().add_modifier(Modifier::UNDERLINED))),
                            Line::from(""),
                            Line::from(lp.text.clone()),
                        ];
                        let details_p = Paragraph::new(details).wrap(Wrap { trim: true });
                        f.render_widget(details_p, chunks[4]);
                    }
                }
            }
        }
    } else {
         let p = Paragraph::new("No game selected").alignment(Alignment::Center);
         f.render_widget(p, inner_area);
    }
}

use ratatui::widgets::Widget;

struct FieldDisplay<'a> {
    home: &'a crate::model::Competitor,
    away: &'a crate::model::Competitor,
    comp: &'a crate::model::Competition,
}

impl<'a> Widget for FieldDisplay<'a> {
    #[allow(deprecated)] // Suppress get_mut warning for stability if cell_mut varies by version
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        if area.width < 20 || area.height < 2 {
            return;
        }

        let home_color = parse_color(self.home.team.color.as_deref().unwrap_or("000000"));
        let away_color = parse_color(self.away.team.color.as_deref().unwrap_or("000000"));

        // Field Dimensions: 0-120 yards (10 EZ + 100 Field + 10 EZ)
        // Map area.width columns to 120 yards.
        
        for x in area.left()..area.right() {
            for y in area.top()..area.bottom() {
                // Determine logical yard (0.0 to 120.0)
                let relative_x = x - area.left();
                let width = area.width as f64;
                let pct = relative_x as f64 / width;
                let logical_yard = pct * 120.0;
                
                let cell = buf.get_mut(x, y);

                // Background Coloring
                if logical_yard < 10.0 {
                    // Away End Zone
                    cell.set_bg(away_color);
                } else if logical_yard > 110.0 {
                    // Home End Zone
                    cell.set_bg(home_color);
                } else {
                    // Field
                    cell.set_bg(Color::Rgb(0, 150, 0)); // Darker Green
                    
                    // 10-yard lines
                    // Just simple lines
                }
            }
        }

        // Draw Yard Lines (White Vertical Lines)
        for yard in (20..=100).step_by(10) {
            let pct = yard as f64 / 120.0;
            let col = area.left() + (pct * area.width as f64) as u16;
            if col < area.right() {
                for y in area.top()..area.bottom() {
                     if let Some(cell) = buf.cell_mut((col, y)) {
                        cell.set_fg(Color::White).set_char('|');
                     }
                }
            }
        }

        // Game Markers
        if let Some(sit) = &self.comp.situation {
            if let Some(yl) = sit.yard_line {
                 let mut field_col = 0;
                 let mut is_away_pos = false;

                 if let Some(pos_id) = &sit.possession {
                     if pos_id == &self.away.team.id.clone().unwrap_or_default() {
                         is_away_pos = true;
                     }
                 }
                 
                 // Logic: 
                 // If Away Poss (L->R): yl is "To Go". 
                 //   If 80 to go (Own 20) -> x = 110 - 80 = 30. Correct.
                 //   If 10 to go (Opp 10) -> x = 110 - 10 = 100. Correct.
                 // If Home Poss (R->L): yl is "To Go".
                 //   If 80 to go (Own 20) -> x = 10 + 80 = 90. Correct.
                 //   If 10 to go (Opp 10) -> x = 10 + 10 = 20. Correct.
                 
                 let logical_loc = if is_away_pos {
                     110.0 - yl as f64
                 } else {
                     10.0 + yl as f64
                 };
                 
                 let pct = logical_loc / 120.0;
                 field_col = area.left() + (pct * area.width as f64) as u16;

                 // Scrimmage Line (White)
                 if field_col < area.right() {
                     for y in area.top()..area.bottom() {
                         if let Some(cell) = buf.cell_mut((field_col, y)) {
                             cell.set_bg(Color::White).set_char(' '); // Solid White Block
                         }
                     }
                 }
                 
                 // First Down Line (Yellow)
                 if let Some(dist) = sit.distance {
                      // Distance is always "Forward".
                      // Away (L->R): forward is +x.
                      // Home (R->L): forward is -x.
                      let fd_loc = if is_away_pos {
                          logical_loc + dist as f64
                      } else {
                          logical_loc - dist as f64
                      };
                      let fd_pct = fd_loc / 120.0;
                      let fd_col = area.left() + (fd_pct * area.width as f64) as u16;
                      
                      if fd_col < area.right() && fd_col != field_col { // Don't overwrite LoS completely if same
                          for y in area.top()..area.bottom() {
                              if let Some(cell) = buf.cell_mut((fd_col, y)) {
                                  cell.set_bg(Color::Yellow).set_char(' '); // Solid Yellow Block
                              }
                          }
                      }
                 }
            }
        }
        
        // Text Labels (Overlaid on End Zones)
        let away_label = &self.away.team.abbreviation;
        let home_label = &self.home.team.abbreviation;
        
        // Simple write at center of EZ
        // Middle Y
        let mid_y = area.top() + area.height / 2;
        
        // Away Label (~Yard 5) -> 5/120 width
        let away_col = area.left() + ((5.0 / 120.0) * area.width as f64) as u16;
        if let Some(cell) = buf.cell_mut((away_col.saturating_sub(1), mid_y)) {
             cell.set_symbol(away_label);
        }
        // Using `set_string` or similar if available, or just writing chars manually?
        // Buffer has `set_string(x, y, string, style)`
        if area.width > 20 {
             buf.set_string(area.left(), mid_y, away_label, Style::default().fg(Color::White).bg(away_color));
             let h_len = home_label.len() as u16;
             buf.set_string(area.right().saturating_sub(h_len), mid_y, home_label, Style::default().fg(Color::White).bg(home_color));
        }

    }
}
