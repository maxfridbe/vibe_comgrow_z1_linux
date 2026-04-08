use crate::gcode;
use crate::icons::*;
use crate::state::{AppState, StringArena};
use crate::styles::*;
use crate::ui::{Section, render_burn_btn, render_jog_btn, render_outline_btn, render_slider};
use arboard::Clipboard;
use clay_layout::layout::{Alignment, LayoutAlignmentX, LayoutAlignmentY, LayoutDirection, Padding};
use clay_layout::{Declaration, fixed, grow};
use raylib::prelude::*;
use std::sync::{Arc, Mutex};

pub fn render_manual_left_subcol<'a, 'render>(
    clay: &mut clay_layout::ClayLayoutScope<'a, 'render, Texture2D, ()>,
    state: &Arc<Mutex<AppState>>,
    sections: &[Section],
    mouse_pressed: bool,
    _clipboard: &mut Option<Clipboard>,
    arena: &StringArena,
    font_scale: f32,
) where
    'a: 'render,
{
    let mut left_col = Declaration::<Texture2D, ()>::new();
    left_col.layout().width(grow!()).height(grow!()).direction(LayoutDirection::TopToBottom).child_gap(16).end();

    let is_idle = { state.lock().unwrap().machine_state == "Idle" };

    clay.with(&left_col, |clay_scope| {
        for section in sections.iter().filter(|s| s.title != "Safety" && s.title != "Test Patterns") {
            let mut section_box = Declaration::<Texture2D, ()>::new();
            section_box
                .layout()
                .width(grow!())
                .padding(Padding::all(6))
                .direction(LayoutDirection::TopToBottom)
                .child_gap(12)
                .end()
                .background_color(COLOR_BG_SECTION)
                .corner_radius()
                .all(16.0 * font_scale)
                .end();

            clay_scope.with(&section_box, |clay_scope| {
                let mut title_line = Declaration::<Texture2D, ()>::new();
                title_line
                    .layout()
                    .child_gap(8)
                    .child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center))
                    .end();
                clay_scope.with(&title_line, |clay_scope| {
                    clay_scope.text(
                        arena.push(format!("{}   {}", section.icon, section.title)),
                        clay_layout::text::TextConfig::new()
                            .font_size((14.0 * font_scale) as u16)
                            .color(section.color)
                            .end(),
                    );
                });

                for chunk in section.commands.chunks(3) {
                    let mut row = Declaration::<Texture2D, ()>::new();
                    row.layout()
                        .width(grow!())
                        .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                        .child_gap(8)
                        .end();
                    clay_scope.with(&row, |clay_scope| {
                        for cmd in chunk {
                            let disabled = !is_idle
                                && cmd.label != "Status"
                                && cmd.label != "Hold"
                                && cmd.label != "Reset"
                                && cmd.label != "Unlock"
                                && cmd.label != "Resume";
                            let btn_id = clay_scope.id(cmd.label);
                            let mut btn_color = if disabled {
                                COLOR_BG_DISABLED
                            } else {
                                COLOR_BG_DARK
                            };
                            let mut text_color = if disabled {
                                COLOR_TEXT_DISABLED
                            } else {
                                COLOR_TEXT_MUTED
                            };
                            if !disabled && clay_scope.pointer_over(btn_id) {
                                btn_color = COLOR_PRIMARY_HOVER;
                                text_color = COLOR_TEXT_WHITE;
                                if mouse_pressed {
                                    let mut guard = state.lock().unwrap();
                                    guard.send_command(cmd.cmd.to_string());
                                }
                            }
                            let mut btn = Declaration::<Texture2D, ()>::new();
                            btn.id(btn_id)
                                .layout()
                                .width(fixed!(110.0 * font_scale))
                                .padding(Padding::all(6))
                                .direction(LayoutDirection::LeftToRight)
                                .child_gap(6)
                                .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                                .end()
                                .background_color(btn_color)
                                .corner_radius()
                                .all(8.0 * font_scale)
                                .end();

                            let subtext_color = if disabled {
                                COLOR_BG_SECTION
                            } else if btn_color == COLOR_PRIMARY_HOVER {
                                COLOR_TEXT_WHITE
                            } else {
                                COLOR_TEXT_DISABLED
                            };
                            clay_scope.with(&btn, |clay_scope| {
                                let mut text_stack = Declaration::<Texture2D, ()>::new();
                                text_stack.layout().direction(LayoutDirection::TopToBottom).child_gap(2).end();
                                clay_scope.with(&text_stack, |clay_scope| {
                                    clay_scope.text(
                                        arena.push(format!("{}   {}", section.icon, cmd.label)),
                                        clay_layout::text::TextConfig::new()
                                            .font_size((12.0 * font_scale) as u16)
                                            .color(text_color)
                                            .end(),
                                    );
                                    clay_scope.text(
                                        arena.push(format!("({})", cmd.cmd)),
                                        clay_layout::text::TextConfig::new()
                                            .font_size((9.0 * font_scale) as u16)
                                            .color(subtext_color)
                                            .end(),
                                    );
                                });
                            });
                        }
                    });
                }
            });
        }
    });
}

pub fn render_manual_right_col<'a, 'render>(
    clay: &mut clay_layout::ClayLayoutScope<'a, 'render, Texture2D, ()>,
    state: &Arc<Mutex<AppState>>,
    _sections: &[Section],
    mouse_pos: raylib::math::Vector2,
    mouse_down: bool,
    mouse_pressed: bool,
    scroll_y: f32,
    clipboard: &mut Option<Clipboard>,
    arena: &StringArena,
    font_scale: f32,
) where
    'a: 'render,
{
    let mut right_col = Declaration::<Texture2D, ()>::new();
    right_col
        .layout()
        .height(grow!())
        .direction(LayoutDirection::TopToBottom)
        .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Top))
        .child_gap(16)
        .width(grow!())
        .end();

    let is_idle = { state.lock().unwrap().machine_state == "Idle" };

    clay.with(&right_col, |clay_scope| {
        // 1. Burn Controls (at the very top)
        let mut burn_box = Declaration::<Texture2D, ()>::new();
        burn_box
            .layout()
            .padding(Padding::all(12))
            .direction(LayoutDirection::TopToBottom)
            .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Top))
            .child_gap(12)
            .end()
            .background_color(COLOR_BG_SECTION)
            .corner_radius()
            .all(16.0 * font_scale)
            .end();

        clay_scope.with(&burn_box, |clay_scope| {
            clay_scope.text(
                "BURN JOG",
                clay_layout::text::TextConfig::new()
                    .font_size((12.0 * font_scale) as u16)
                    .color(COLOR_TEXT_LABEL)
                    .end(),
            );

            let mut r1 = Declaration::<Texture2D, ()>::new();
            r1.layout()
                .child_gap(8)
                .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                .end();
            clay_scope.with(&r1, |clay_scope| {
                let mut btn_box = Declaration::<Texture2D, ()>::new();
                btn_box.layout().direction(LayoutDirection::LeftToRight).child_gap(2).end();
                clay_scope.with(&btn_box, |clay_scope| {
                    render_burn_btn(
                        clay_scope,
                        "b_ul",
                        "NW",
                        state,
                        -1.0,
                        1.0,
                        mouse_pressed,
                        clipboard,
                        arena,
                        font_scale,
                        !is_idle,
                    );
                });

                let mut btn_box = Declaration::<Texture2D, ()>::new();
                btn_box.layout().direction(LayoutDirection::LeftToRight).child_gap(2).end();
                clay_scope.with(&btn_box, |clay_scope| {
                    render_burn_btn(
                        clay_scope,
                        "b_up",
                        "NORTH",
                        state,
                        0.0,
                        1.0,
                        mouse_pressed,
                        clipboard,
                        arena,
                        font_scale,
                        !is_idle,
                    );
                });

                let mut btn_box = Declaration::<Texture2D, ()>::new();
                btn_box.layout().direction(LayoutDirection::LeftToRight).child_gap(2).end();
                clay_scope.with(&btn_box, |clay_scope| {
                    render_burn_btn(
                        clay_scope,
                        "b_ur",
                        "NE",
                        state,
                        1.0,
                        1.0,
                        mouse_pressed,
                        clipboard,
                        arena,
                        font_scale,
                        !is_idle,
                    );
                });
            });
            let mut r2 = Declaration::<Texture2D, ()>::new();
            r2.layout()
                .child_gap(8)
                .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                .end();
            clay_scope.with(&r2, |clay_scope| {
                let mut btn_box = Declaration::<Texture2D, ()>::new();
                btn_box.layout().direction(LayoutDirection::LeftToRight).child_gap(2).end();
                clay_scope.with(&btn_box, |clay_scope| {
                    render_burn_btn(
                        clay_scope,
                        "b_l",
                        "WEST",
                        state,
                        -1.0,
                        0.0,
                        mouse_pressed,
                        clipboard,
                        arena,
                        font_scale,
                        !is_idle,
                    );
                });

                let mut fire_box = Declaration::<Texture2D, ()>::new();
                fire_box
                    .layout()
                    .child_gap(4)
                    .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                    .end();
                clay_scope.with(&fire_box, |clay_scope| {
                    let btn_id = clay_scope.id("fire_btn");
                    let mut btn_color = if !is_idle {
                        COLOR_BG_DISABLED
                    } else {
                        COLOR_DANGER
                    };
                    let mut fire_text_color = if !is_idle {
                        COLOR_TEXT_DISABLED
                    } else {
                        COLOR_TEXT_WHITE
                    };
                    if is_idle && clay_scope.pointer_over(btn_id) {
                        btn_color = COLOR_DANGER_BRIGHT;
                        fire_text_color = COLOR_TEXT_WHITE;
                        if mouse_pressed {
                            let mut guard = state.lock().unwrap();
                            let s = guard.power;
                            guard.send_command(gcode::laser_on(s));
                        }
                    }
                    let mut fire_btn = Declaration::<Texture2D, ()>::new();
                    fire_btn
                        .id(btn_id)
                        .layout()
                        .width(fixed!(50.0 * font_scale))
                        .padding(Padding::all(4))
                        .direction(LayoutDirection::LeftToRight)
                        .child_gap(4)
                        .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                        .end()
                        .background_color(btn_color)
                        .corner_radius()
                        .all(8.0 * font_scale)
                        .end();

                    clay_scope.with(&fire_btn, |clay| {
                        clay.text(
                            arena.push(format!("{}   FIRE", ICON_FLAME)),
                            clay_layout::text::TextConfig::new()
                                .font_size((10.0 * font_scale) as u16)
                                .color(fire_text_color)
                                .end(),
                        );
                    });

                    let off_id = clay_scope.id("burn_off_btn");
                    let mut off_color = COLOR_BG_DARK;
                    let mut off_text_color = COLOR_TEXT_WHITE;
                    if clay_scope.pointer_over(off_id) {
                        off_color = COLOR_PRIMARY_HOVER;
                        off_text_color = COLOR_TEXT_WHITE;
                        if mouse_pressed {
                            state.lock().unwrap().send_command(gcode::CMD_LASER_OFF.to_string());
                        }
                    }
                    let mut off_btn = Declaration::<Texture2D, ()>::new();
                    off_btn
                        .id(off_id)
                        .layout()
                        .width(fixed!(50.0 * font_scale))
                        .padding(Padding::all(4))
                        .direction(LayoutDirection::LeftToRight)
                        .child_gap(4)
                        .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                        .end()
                        .background_color(off_color)
                        .corner_radius()
                        .all(8.0 * font_scale)
                        .end();
                    clay_scope.with(&off_btn, |clay| {
                        clay.text(
                            arena.push(format!("{}   OFF", ICON_POWER)),
                            clay_layout::text::TextConfig::new()
                                .font_size((10.0 * font_scale) as u16)
                                .color(off_text_color)
                                .end(),
                        );
                    });
                });

                let mut btn_box = Declaration::<Texture2D, ()>::new();
                btn_box.layout().direction(LayoutDirection::LeftToRight).child_gap(2).end();
                clay_scope.with(&btn_box, |clay_scope| {
                    render_burn_btn(
                        clay_scope,
                        "b_r",
                        "EAST",
                        state,
                        1.0,
                        0.0,
                        mouse_pressed,
                        clipboard,
                        arena,
                        font_scale,
                        !is_idle,
                    );
                });
            });
            let mut r3 = Declaration::<Texture2D, ()>::new();
            r3.layout()
                .child_gap(8)
                .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                .end();
            clay_scope.with(&r3, |clay_scope| {
                let mut btn_box = Declaration::<Texture2D, ()>::new();
                btn_box.layout().direction(LayoutDirection::LeftToRight).child_gap(2).end();
                clay_scope.with(&btn_box, |clay_scope| {
                    render_burn_btn(
                        clay_scope,
                        "b_dl",
                        "SW",
                        state,
                        -1.0,
                        -1.0,
                        mouse_pressed,
                        clipboard,
                        arena,
                        font_scale,
                        !is_idle,
                    );
                });

                let mut btn_box = Declaration::<Texture2D, ()>::new();
                btn_box.layout().direction(LayoutDirection::LeftToRight).child_gap(2).end();
                clay_scope.with(&btn_box, |clay_scope| {
                    render_burn_btn(
                        clay_scope,
                        "b_dn",
                        "SOUTH",
                        state,
                        0.0,
                        -1.0,
                        mouse_pressed,
                        clipboard,
                        arena,
                        font_scale,
                        !is_idle,
                    );
                });

                let mut btn_box = Declaration::<Texture2D, ()>::new();
                btn_box.layout().direction(LayoutDirection::LeftToRight).child_gap(2).end();
                clay_scope.with(&btn_box, |clay_scope| {
                    render_burn_btn(
                        clay_scope,
                        "b_dr",
                        "SE",
                        state,
                        1.0,
                        -1.0,
                        mouse_pressed,
                        clipboard,
                        arena,
                        font_scale,
                        !is_idle,
                    );
                });
            });

            // Descriptive Laser Mode Buttons
            let mut mode_row = Declaration::<Texture2D, ()>::new();
            mode_row
                .layout()
                .child_gap(8)
                .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                .padding(Padding::vertical(8))
                .end();
            clay_scope.with(&mode_row, |clay| {
                let modes: [(&str, &str, &str); 3] = [
                    ("LASER CONST", gcode::CMD_LASER_CONST, ICON_FLAME),
                    ("LASER DYN", gcode::CMD_LASER_DYN, ICON_REFRESH),
                    ("LASER OFF", gcode::CMD_LASER_OFF, ICON_POWER),
                ];
                for (label, cmd, icon) in modes {
                    let disabled = !is_idle && label != "LASER OFF";
                    let btn_id = clay.id(label);
                    let mut btn_color = if disabled {
                        COLOR_BG_DISABLED
                    } else {
                        COLOR_BG_DARK
                    };
                    let mut btn_text_color = if disabled {
                        COLOR_TEXT_DISABLED
                    } else {
                        COLOR_TEXT_MUTED
                    };
                    if !disabled && clay.pointer_over(btn_id) {
                        btn_color = COLOR_PRIMARY_HOVER;
                        btn_text_color = COLOR_TEXT_WHITE;
                        if mouse_pressed {
                            state.lock().unwrap().send_command(cmd.to_string());
                        }
                    }
                    let mut btn = Declaration::<Texture2D, ()>::new();
                    btn.id(btn_id)
                        .layout()
                        .width(fixed!(90.0 * font_scale))
                        .padding(Padding::all(6))
                        .direction(LayoutDirection::LeftToRight)
                        .child_gap(4)
                        .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                        .end()
                        .background_color(btn_color)
                        .corner_radius()
                        .all(8.0 * font_scale)
                        .end();

                    clay.with(&btn, |clay| {
                        clay.text(
                            arena.push(format!("{} {}", icon, label)),
                            clay_layout::text::TextConfig::new()
                                .font_size((9.0 * font_scale) as u16)
                                .color(btn_text_color)
                                .end(),
                        );
                    });
                }
            });
        });

        // 2. Jog Grid
        let mut jog_box = Declaration::<Texture2D, ()>::new();
        jog_box
            .layout()
            .padding(Padding::all(12))
            .direction(LayoutDirection::TopToBottom)
            .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Top))
            .child_gap(16)
            .end()
            .background_color(COLOR_BG_SECTION)
            .corner_radius()
            .all(16.0 * font_scale)
            .end();
        clay_scope.with(&jog_box, |clay_scope| {
            let mut jog_grid = Declaration::<Texture2D, ()>::new();
            jog_grid
                .layout()
                .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                .direction(LayoutDirection::TopToBottom)
                .child_gap(8)
                .end();
            clay_scope.with(&jog_grid, |clay_scope| {
                let mut row1 = Declaration::<Texture2D, ()>::new();
                row1.layout().child_gap(8).end();
                clay_scope.with(&row1, |clay_scope| {
                    render_jog_btn(
                        clay_scope,
                        "up",
                        crate::icons::ICON_ARROW_UP,
                        state,
                        "Y",
                        1.0,
                        mouse_pressed,
                        clipboard,
                        font_scale,
                        !is_idle,
                    );
                });

                let mut row2 = Declaration::<Texture2D, ()>::new();
                row2.layout()
                    .child_gap(8)
                    .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                    .end();
                clay_scope.with(&row2, |clay_scope| {
                    render_jog_btn(
                        clay_scope,
                        "left",
                        crate::icons::ICON_ARROW_LEFT,
                        state,
                        "X",
                        -1.0,
                        mouse_pressed,
                        clipboard,
                        font_scale,
                        !is_idle,
                    );

                    let center_id = clay_scope.id("center");
                    let mut center_color = if !is_idle {
                        COLOR_BG_DISABLED
                    } else {
                        COLOR_TEXT_BLACK
                    };
                    if is_idle && clay_scope.pointer_over(center_id) {
                        center_color = COLOR_BG_SECTION;
                        if mouse_pressed {
                            let mut guard = state.lock().unwrap();
                            guard.v_pos = raylib::prelude::Vector2::new(0.0, 0.0);
                            guard.send_command(gcode::CMD_SET_ORIGIN.to_string());
                            if let Some(cb) = clipboard {
                                let _ = cb.set_text(gcode::CMD_SET_ORIGIN.to_string());
                            }
                        }
                    }
                    let mut center_btn = Declaration::<Texture2D, ()>::new();
                    center_btn
                        .id(center_id)
                        .layout()
                        .width(fixed!(30.0 * font_scale))
                        .height(fixed!(30.0 * font_scale))
                        .padding(Padding::all(4))
                        .direction(LayoutDirection::TopToBottom)
                        .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                        .end()
                        .background_color(center_color)
                        .corner_radius()
                        .all(8.0 * font_scale)
                        .end();

                    let icon_color = if !is_idle {
                        COLOR_BG_SECTION
                    } else {
                        COLOR_PRIMARY
                    };
                    clay_scope.with(&center_btn, |clay_scope| {
                        clay_scope.text(
                            crate::icons::ICON_CROSSHAIR,
                            clay_layout::text::TextConfig::new()
                                .font_size((24.0 * font_scale) as u16)
                                .color(icon_color)
                                .end(),
                        );
                    });

                    let home_zero_id = clay_scope.id("home_zero");
                    let mut home_zero_color = if !is_idle {
                        COLOR_BG_DISABLED
                    } else {
                        COLOR_TEXT_BLACK
                    };
                    if is_idle && clay_scope.pointer_over(home_zero_id) {
                        home_zero_color = COLOR_BG_SECTION;
                        if mouse_pressed {
                            let mut guard = state.lock().unwrap();
                            guard.v_pos = raylib::prelude::Vector2::new(0.0, 0.0);
                            let cmd = format!("{} {}", gcode::CMD_ABSOLUTE_POS, gcode::move_xy(0.0, 0.0));
                            guard.send_command(cmd.clone());
                            if let Some(cb) = clipboard {
                                let _ = cb.set_text(cmd);
                            }
                        }
                    }
                    let mut home_zero_btn = Declaration::<Texture2D, ()>::new();
                    home_zero_btn
                        .id(home_zero_id)
                        .layout()
                        .width(fixed!(30.0 * font_scale))
                        .height(fixed!(30.0 * font_scale))
                        .padding(Padding::all(4))
                        .direction(LayoutDirection::TopToBottom)
                        .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                        .end()
                        .background_color(home_zero_color)
                        .corner_radius()
                        .all(8.0 * font_scale)
                        .end();

                    let home_icon_color = if !is_idle {
                        COLOR_BG_SECTION
                    } else {
                        COLOR_SUCCESS_LIGHT
                    };
                    clay_scope.with(&home_zero_btn, |clay_scope| {
                        clay_scope.text(
                            crate::icons::ICON_HOME,
                            clay_layout::text::TextConfig::new()
                                .font_size((24.0 * font_scale) as u16)
                                .color(home_icon_color)
                                .end(),
                        );
                    });

                    render_jog_btn(
                        clay_scope,
                        "right",
                        crate::icons::ICON_ARROW_RIGHT,
                        state,
                        "X",
                        1.0,
                        mouse_pressed,
                        clipboard,
                        font_scale,
                        !is_idle,
                    );
                });

                let mut row3 = Declaration::<Texture2D, ()>::new();
                row3.layout().child_gap(8).end();
                clay_scope.with(&row3, |clay_scope| {
                    render_jog_btn(
                        clay_scope,
                        "down",
                        crate::icons::ICON_ARROW_DOWN,
                        state,
                        "Y",
                        -1.0,
                        mouse_pressed,
                        clipboard,
                        font_scale,
                        !is_idle,
                    );
                });
            });
        });

        // 3. Sliders
        let mut sliders_box = Declaration::<Texture2D, ()>::new();
        sliders_box
            .layout()
            .padding(Padding::all(12))
            .direction(LayoutDirection::TopToBottom)
            .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Top))
            .child_gap(16)
            .end()
            .background_color(COLOR_BG_SECTION)
            .corner_radius()
            .all(16.0 * font_scale)
            .end();
        clay_scope.with(&sliders_box, |clay_scope| {
            let (dist, feed, pwr) = {
                let g = state.lock().unwrap();
                (g.distance, g.feed_rate, g.power)
            };
            render_slider(
                clay_scope,
                "dist_slider",
                "Step",
                dist,
                0.1,
                100.0,
                COLOR_SLIDER_STEP,
                state,
                |s, v| s.distance = v,
                mouse_pos,
                mouse_down,
                scroll_y,
                arena,
                font_scale,
            );
            render_slider(
                clay_scope,
                "feed_slider",
                "Speed",
                feed,
                10.0,
                6000.0,
                COLOR_SLIDER_SPEED,
                state,
                |s, v| s.feed_rate = v,
                mouse_pos,
                mouse_down,
                scroll_y,
                arena,
                font_scale,
            );
            render_slider(
                clay_scope,
                "power_slider",
                "Power",
                pwr,
                0.0,
                1000.0,
                COLOR_SLIDER_POWER,
                state,
                |s, v| s.power = v,
                mouse_pos,
                mouse_down,
                scroll_y,
                arena,
                font_scale,
            );
        });
    });
}
