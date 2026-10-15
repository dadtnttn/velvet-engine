//! Fuera: el Bosque Oscuro y la villa, que comparten el cielo, la Reina y el paseo libre.
use super::*;

impl RenderState {
    /// Out of doors: the Dark Forest, or the villa she has walked into; the same Queen.
    pub(super) fn render_outdoors(&mut self, frame: &Frame, bob: f32, green_outfit: bool) {
        let Frame {
            camera,
            player,
            yaw,
            elapsed,
            footprints,
            queen_thought,
            nature_mode,
            village_entered,
            map_open,
            wardrobe_open,
            wardrobe_selection,
            outfit,
            shop_open,
            shop_tab,
            shop_category,
            shop_selection,
            shop_quantity,
            gold,
            inventory,
            shop_stock,
            shop_notice,
            shop_pointer,
            shop_preview_rotation,
            venue,
            environment,
            action_prompt,
            ..
        } = *frame;
        let player_ground_y = if nature_mode && village_entered {
            village::surface_height(player.x, player.z)
        } else if nature_mode {
            nature_walk_surface_height(player.x, player.z)
        } else {
            0.0
        };
        let actor_lift = if nature_mode { 0.018 } else { 0.0 };
        let queen_matrix = Mat4::from_translation(Vec3::new(
            player.x,
            player_ground_y + actor_lift + bob,
            player.z,
        )) * Mat4::from_rotation_y(yaw)
            * Mat4::from_scale(Vec3::new(
                QUEEN_WORLD_SCALE,
                QUEEN_WORLD_SCALE,
                QUEEN_WORLD_SCALE,
            ));
        let mut draws = Vec::with_capacity(
            if green_outfit {
                self.green_queen_meshes.len()
            } else {
                self.queen_meshes.len()
            } + self.mage_meshes.len()
                + self.maid_meshes.len()
                + self.maid_alt_meshes.len()
                + self.nature_draws.len()
                + self.fantasy_village_draws.len()
                + footprints.len()
                + 18,
        );
        if nature_mode {
            for footprint in footprints {
                let fade = (1.0 - footprint.age / 12.0).clamp(0.0, 1.0);
                // Sit just above the shared height field. The former raised
                // dirt ribbon is gone, so prints never float over a second
                // surface.
                let y = if village_entered {
                    village::surface_height(footprint.position.x, footprint.position.z) + 0.022
                } else {
                    nature_walk_surface_height(footprint.position.x, footprint.position.z) + 0.022
                };
                draws.push(MeshDraw3D {
                    mesh: self.nature_footprint,
                    transform: Transform3D {
                        position: Vec3::new(footprint.position.x, y, footprint.position.z),
                        rotation: Vec3::new(0.0, footprint.rotation_y, 0.0),
                        scale: Vec3::new(
                            footprint.scale * QUEEN_FOOTPRINT_WIDTH_SCALE,
                            1.0,
                            footprint.scale * QUEEN_FOOTPRINT_LENGTH_SCALE,
                        ),
                    }
                    .matrix(),
                    material: Material3D {
                        // Warm damp soil contrasts with the green map while
                        // retaining enough alpha to feel pressed into it.
                        base_color: [0.145, 0.040, 0.010, fade * 0.88],
                        texture: None,
                        occlusion_window: [0.0; 4],
                    },
                    texture: None,
                });
            }
        }
        draws.push(MeshDraw3D {
            mesh: self.player_marker,
            transform: Mat4::from_translation(Vec3::new(
                player.x,
                player_ground_y + actor_lift + 0.026,
                player.z,
            )),
            material: Material3D {
                base_color: [0.42, 0.12, 0.82, 0.9],
                texture: None,
                occlusion_window: [0.0; 4],
            },
            texture: None,
        });
        if nature_mode {
            let celestial = environment.celestial;
            let sky_target = camera.target;
            let orb_distance = 34.0;
            // Compress the vertical component so the celestial bodies stay
            // inside the isometric camera's compact orthographic sky instead
            // of flying above the viewport at noon or midnight.
            let sky_direction = |direction: Vec3| {
                Vec3::new(direction.x, direction.y * 0.42, direction.z).normalize_or_zero()
            };
            if celestial.sun_visibility > 0.02 {
                let sunset = celestial.horizon_strength.clamp(0.0, 1.0);
                draws.push(MeshDraw3D {
                    mesh: self.celestial_orb,
                    transform: Transform3D {
                        position: sky_target
                            + sky_direction(celestial.sun_direction) * orb_distance,
                        scale: {
                            let size = 1.35 + sunset * 0.28;
                            Vec3::new(size, size, size)
                        },
                        ..Default::default()
                    }
                    .matrix(),
                    material: Material3D {
                        base_color: [1.60, 0.72 + sunset * 0.20, 0.20 + sunset * 0.08, 1.0],
                        texture: None,
                        occlusion_window: [0.0; 4],
                    },
                    texture: None,
                });
            }
            if celestial.moon_visibility > 0.02 {
                let lunar_illumination =
                    0.5 - 0.5 * (celestial.moon_phase * std::f32::consts::TAU).cos();
                let glow = 0.36 + lunar_illumination * 0.84;
                draws.push(MeshDraw3D {
                    mesh: self.celestial_orb,
                    transform: Transform3D {
                        position: sky_target
                            + sky_direction(celestial.moon_direction) * orb_distance,
                        scale: {
                            let size = 0.88 + lunar_illumination * 0.20;
                            Vec3::new(size, size, size)
                        },
                        ..Default::default()
                    }
                    .matrix(),
                    material: Material3D {
                        base_color: [
                            celestial.moon_color[0] * glow,
                            celestial.moon_color[1] * glow,
                            celestial.moon_color[2] * glow,
                            1.0,
                        ],
                        texture: None,
                        occlusion_window: [0.0; 4],
                    },
                    texture: None,
                });
            }
        }
        for (mesh, material, texture, local) in self.queen_parts(outfit) {
            draws.push(MeshDraw3D {
                mesh,
                transform: queen_matrix * local,
                material: material_for_outfit(material, outfit).with_ambient_occlusion(0.28),
                texture,
            });
        }
        if village_entered {
            self.village_draws(frame, &mut draws, player_ground_y);
        } else {
            self.forest_draws(frame, &mut draws, player_ground_y);
        }
        let environment = if village_entered {
            village::mood(environment)
        } else {
            dark_forest::mood(environment)
        };
        let local_lights = if village_entered {
            village::lights(elapsed, environment.celestial.moon_visibility, player)
        } else {
            dark_forest::lights(elapsed, environment.celestial.moon_visibility, player)
        };
        // The villa keeps the prototype's toon finish (ink lines, richer colour) and relief; every
        // other scene gets the renderer's own back after this frame.
        let look = if nature_mode && village_entered {
            Some((village::finish(), village::toon_style()))
        } else if nature_mode {
            Some((dark_forest::finish(), dark_forest::toon_style()))
        } else {
            None
        };
        let villa_look = look.map(|(finish, toon)| {
            (
                self.renderer.replace_finish(finish),
                self.renderer.replace_toon_style(toon),
            )
        });
        if let Some(guide) = frame.mission {
            self.mission_draws(guide, elapsed, &mut draws);
        }
        let (scene_view, scene_size) = self.scene_target();
        self.renderer.render_with_environment_and_lights(
            &self.device,
            &self.queue,
            &scene_view,
            scene_size,
            camera,
            environment,
            &local_lights,
            &draws,
        );
        self.finish_scene(&scene_view, scene_size);
        if let Some((finish, toon)) = villa_look {
            self.renderer.replace_finish(finish);
            self.renderer.replace_toon_style(toon);
        }
        self.map_overlay.set_markers(take_world_markers(camera));
        let view = &self.target_view;
        let speech_bubbles = npc_speech_bubbles(
            camera,
            player,
            elapsed,
            nature_mode,
            village_entered,
            map_open,
            wardrobe_open,
            queen_thought,
        );
        self.map_overlay.draw(
            &self.device,
            &self.queue,
            view,
            (WINDOW_WIDTH, WINDOW_HEIGHT),
            player,
            yaw,
            nature_mode,
            village_entered,
            map_open,
            wardrobe_open,
            wardrobe_selection,
            outfit,
            shop_open,
            shop_tab,
            shop_category,
            shop_selection,
            shop_quantity,
            gold,
            inventory,
            shop_stock,
            shop_notice,
            shop_pointer,
            elapsed,
            shop_preview_rotation,
            &speech_bubbles,
            Some(venue),
            action_prompt,
            None,
            false,
        );
    }
}

impl App {
    /// One step out of doors (the wood or the villa): walking where the view points, the solids
    /// of the place, the corners eased, her steps heard and her prints left behind.
    pub(super) fn walk_outdoors(&mut self, dt: f32) {
        // Movement follows the current view, so W always advances toward the
        // top of the screen and A/D strafe left/right after every camera turn.
        // Use the target yaw here so controls respond immediately, while the
        // rendered camera eases into place to avoid a disorienting snap.
        let (view_forward, view_right) = self.iso.walk_axes();
        let mut direction = Vec3::ZERO;
        if self.move_forward {
            direction += view_forward;
        }
        if self.move_back {
            direction = direction - view_forward;
        }
        if self.move_left {
            direction = direction - view_right;
        }
        if self.move_right {
            direction += view_right;
        }
        let moving = direction.length_squared() > 0.0;
        let previous_player = self.player;
        if moving {
            let direction = direction.normalize_or_zero();
            let move_speed = if self.sprint { 4.6 } else { 2.2 };
            let movement = direction * (dt * move_speed);
            let (min_x, max_x, min_z, max_z) = if self.village_entered {
                let c = fantasy_village_center();
                (
                    c.x - village::RADIUS,
                    c.x + village::RADIUS,
                    c.z - village::RADIUS,
                    c.z + village::RADIUS,
                )
            } else {
                (
                    -NATURE_PLAYABLE_LIMIT,
                    NATURE_PLAYABLE_LIMIT,
                    -NATURE_PLAYABLE_LIMIT,
                    NATURE_PLAYABLE_LIMIT,
                )
            };
            let candidate_x = Vec3::new(
                (self.player.x + movement.x).clamp(min_x, max_x),
                0.0,
                self.player.z,
            );
            let x_blocked = if self.village_entered {
                fantasy_village_position_blocked(candidate_x)
            } else {
                nature_position_blocked(candidate_x)
            };
            if !x_blocked {
                self.player.x = candidate_x.x;
            }
            let candidate_z = Vec3::new(
                self.player.x,
                0.0,
                (self.player.z + movement.z).clamp(min_z, max_z),
            );
            let z_blocked = if self.village_entered {
                fantasy_village_position_blocked(candidate_z)
            } else {
                nature_position_blocked(candidate_z)
            };
            if !z_blocked {
                self.player.z = candidate_z.z;
            }
            // Caught on a corner (both axes blocked, or the slide that was left took her almost
            // nowhere): ease round it.
            let slid = Vec3::new(
                self.player.x - previous_player.x,
                0.0,
                self.player.z - previous_player.z,
            )
            .length();
            if (x_blocked && z_blocked) || slid < 0.2 * movement.length() {
                let blocked = |p: Vec3| {
                    let p = Vec3::new(p.x.clamp(min_x, max_x), 0.0, p.z.clamp(min_z, max_z));
                    if self.village_entered {
                        fantasy_village_position_blocked(p)
                    } else {
                        nature_position_blocked(p)
                    }
                };
                let here = self.player;
                if let Some((ax, az)) = corner_assist(movement.x, movement.z, |ax, az| {
                    !blocked(Vec3::new(here.x + ax, here.y, here.z + az))
                }) {
                    self.player.x = here.x + ax;
                    self.player.z = here.z + az;
                }
            }
            // The native Mixamo Queen faces +Z. atan2(X, Z) maps all four
            // movement directions without mirroring A and D.
            self.player_yaw = direction.x.atan2(direction.z);

            // Steps you can hear, one per stride, by the ground underfoot.
            let stepped = Vec3::new(
                self.player.x - previous_player.x,
                0.0,
                self.player.z - previous_player.z,
            )
            .length();
            self.step_sound_distance += stepped;
            let stride = if self.sprint { 1.5 } else { 1.05 };
            if self.step_sound_distance >= stride {
                self.step_sound_distance -= stride;
                self.play_step();
            }

            if self.nature_mode {
                let displacement = Vec3::new(
                    self.player.x - previous_player.x,
                    0.0,
                    self.player.z - previous_player.z,
                )
                .length();
                self.footstep_distance += displacement;
                while self.footstep_distance >= 0.56 {
                    self.footstep_distance -= 0.56;
                    let lateral = Vec3::new(-direction.z, 0.0, direction.x);
                    // Keep alternating feet.
                    let side = self.footstep_side;
                    let side_offset =
                        QUEEN_FOOTPRINT_SIDE_OFFSET + (self.footprints.len() % 3) as f32 * 0.008;
                    let step_position =
                        self.player - direction * 0.30 + lateral * (side_offset * side);
                    self.footprints.push(Footprint {
                        position: step_position,
                        rotation_y: self.player_yaw + side * 0.055,
                        age: 0.0,
                        scale: 1.0 + (self.footprints.len() % 4) as f32 * 0.025,
                    });
                    if self.footprints.len() > 30 {
                        self.footprints.remove(0);
                    }
                    self.footstep_side = -self.footstep_side;
                }
            }
        }
        let zone = self.zone();
        self.release_stale_prefetches(zone.id());
        zone.prefetch(self);
        // Keep the camera focus exactly on the gameplay position. A lagged
        // focus makes the Queen drift away from the centre at close zoom and,
        // more importantly, puts the reveal window on the wrong screen pixel
        // while she is crossing a wall junction.
        let desired_focus = Vec3::new(self.player.x, 0.0, self.player.z);
        self.camera_focus = desired_focus;
        self.elapsed += dt;
        self.fps_elapsed += dt;
        self.fps_frames = self.fps_frames.saturating_add(1);
        if self.fps_elapsed >= 0.5 {
            self.fps_display = self.fps_frames as f32 / self.fps_elapsed.max(0.001);
            self.fps_elapsed = 0.0;
            self.fps_frames = 0;
        }
    }
}
