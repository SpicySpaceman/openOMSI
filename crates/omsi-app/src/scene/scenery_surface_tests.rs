//! Render queue selection must not give upright scenery ground-surface behavior.
use super::*;

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "openomsi-scenery-surface-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let fixture = Self(dir);
        fixture.write(
            "global.cfg",
            "[name]\nScenery surface regression\n[map]\n0\n0\ntile_0_0.map\n",
        );
        fixture.write("sign.x", &Self::plate(0.5, 0.5, "1;0;0;1"));
        fixture.write("pole.x", &Self::plate(0.08, 0.9, "0;1;0;1"));
        fixture.write(
            "sign.sco",
            "[mesh]\nsign.x\n[matl]\nwhite.png\n0\n[matl_alpha]\n1\n",
        );
        let pole = "[collision_mesh]\npole.x\n[crashmode_pole]\n0.02\n0.7\n[LOD]\n0.1\n[mesh]\npole.x\n[shadow]\n[LOD]\n0\n[mesh]\npole.x\n[shadow]\n";
        fixture.write("lamp.sco", &format!("[rendertype]\n3\n{pole}"));
        fixture.write("post.sco", pole);
        fixture.write(
            "surface.sco",
            &format!("[rendertype]\n3\n[surface]\n{pole}"),
        );
        fixture.write(
            "canopy.x",
            "xof 0303txt 0032\nMesh canopy {4;0;0;0;,5;0;0;,5;0;5;,0;0;5;;2;3;0,2,1;,3;0,3,2;;}\n",
        );
        fixture.write("canopy.sco", "[rendertype]\n3\n[mesh]\ncanopy.x\n");
        fixture.write("deck.sco", "[surface]\n[mesh]\ncanopy.x\n");
        image::save_buffer(
            fixture.0.join("white.png"),
            &[255; 4],
            1,
            1,
            image::ColorType::Rgba8,
        )
        .unwrap();
        let mut tile = "[version]\n14\n".to_string();
        for (id, file, x, y) in [
            (1, "sign.sco", 100.0, 105.0),
            (2, "lamp.sco", 100.0, 105.004),
            (3, "surface.sco", 120.0, 105.0),
            (4, "post.sco", 80.0, 105.0),
            (5, "canopy.sco", 130.0, 105.0),
            (6, "deck.sco", 150.0, 105.0),
        ] {
            tile.push_str(&format!(
                "[object]\n0\n{file}\n{id}\n{x}\n{y}\n2\n0\n0\n0\n0\n"
            ));
        }
        fixture.write("tile_0_0.map", &tile);
        fixture
    }

    fn write(&self, name: &str, text: &str) {
        std::fs::write(self.0.join(name), text).unwrap();
    }

    fn plate(w: f32, h: f32, color: &str) -> String {
        format!(
            r#"xof 0303txt 0032
Mesh plate {{
 8; -{w};-{h};0;, {w};-{h};0;, {w};{h};0;, -{w};{h};0;,
    -{w};-{h};0;, {w};-{h};0;, {w};{h};0;, -{w};{h};0;;
 4; 3;0,2,1;, 3;0,3,2;, 3;4,5,6;, 3;4,6,7;;
 MeshTextureCoords {{8;0;0;,1;0;,1;1;,0;1;,0;0;,1;0;,1;1;,0;1;;}}
 MeshMaterialList {{1;4;0,0,0,0;; Material {{{color};;0;0;0;0;;0;0;0;; TextureFilename {{"white.png";}} }} }}
}}
"#
        )
    }

    fn world(&self) -> World {
        World::open(&self.0, &self.0.join("global.cfg"), 20261005).unwrap()
    }

    fn prepare(&self, world: &World) -> Prepared {
        let (mut tiles, _) = world.prepare_tiles(&[(0, 0, self.0.join("tile_0_0.map"))]);
        assert_eq!(tiles.len(), 1);
        tiles.pop().unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn ground_classification_is_independent_of_numeric_render_queues() {
    for (queue, ground) in [
        ("0", false),
        ("1", false),
        ("3", false),
        ("4", false),
        ("presurface", true),
        ("surface", true),
        ("on_surface", true),
    ] {
        for explicit in [false, true] {
            let text = format!(
                "[rendertype]\n{queue}\n{}",
                if explicit { "[surface]\n" } else { "" }
            );
            let sco = SceneryObject::parse(&omsi_cfg::CfgFile::from_str("queue.sco", &text));
            assert_eq!(
                scenery_ground_surface(&sco),
                ground || explicit,
                "queue {queue}, [surface]={explicit}"
            );
        }
    }
}

#[test]
fn paint_classification_is_independent_of_numeric_render_queues() {
    let mesh = |heights: &[f32]| {
        (
            MeshData {
                positions: heights
                    .iter()
                    .enumerate()
                    .map(|(i, z)| glam::Vec3::new(i as f32, 0.0, *z))
                    .collect(),
                ..Default::default()
            },
            Vec::new(),
            Vec::new(),
        )
    };
    for queue in ["0", "1", "3", "4", "presurface", "surface", "on_surface"] {
        for explicit in [false, true] {
            let text = format!(
                "[rendertype]\n{queue}\n{}",
                if explicit { "[surface]\n" } else { "" }
            );
            let sco = SceneryObject::parse(&omsi_cfg::CfgFile::from_str("paint.sco", &text));
            assert_eq!(
                paint_at_foot(&sco, &[mesh(&[0.0, 0.0, 0.0])]),
                !scenery_ground_surface(&sco),
                "flat paint, queue {queue}, [surface]={explicit}"
            );
            assert!(!paint_at_foot(&sco, &[mesh(&[0.0, 0.0, 9.77])]));
        }
    }
}

#[test]
#[ignore = "requires a graphics adapter; exercises production paint upload and road depth"]
fn flat_numeric_queue_paint_stays_visible_over_a_road() {
    let instance = crate::graphics_instance();
    let mut renderer = pollster::block_on(Renderer::new_with(
        &instance,
        None,
        Some(wgpu::TextureFormat::Rgba8UnormSrgb),
        omsi_render::RenderOptions {
            msaa: 1,
            ssao: false,
            shadow_size: 1024,
            fxaa: false,
            render_scale: 1.0,
            ..Default::default()
        },
    ))
    .expect("test renderer");
    let camera = omsi_render::Camera {
        position: DVec3::new(100.0, 100.0, 4.5),
        yaw: 0.0,
        pitch: -25.0,
        roll: 0.0,
        fov_deg: 60.0,
        near: 0.1,
        far: 100.0,
    };
    let lighting = omsi_render::Lighting {
        shadows: false,
        fog_density: 0.0,
        ..Default::default()
    };
    let plate = |w: f32, color: &str| {
        format!(
            "xof 0303txt 0032\nMesh paint {{4;-{w};0;2;,{w};0;2;,{w};0;12;,-{w};0;12;;4;3;0,1,2;,3;0,2,3;,3;0,2,1;,3;0,3,2;;\nMeshTextureCoords {{4;0;0;,1;0;,1;1;,0;1;;}}\nMeshMaterialList {{1;4;0,0,0,0;;Material {{{color};;0;0;0;0;;0;0;0;;TextureFilename {{\"white.png\";}}}}}}}}\n"
        )
    };
    for (queue, phase) in [
        ("0", RenderPhase::Normal),
        ("1", RenderPhase::BeforeNormal),
        ("3", RenderPhase::AfterNormal),
        ("4", RenderPhase::AfterVehicles),
    ] {
        for alpha in [0, 2] {
            let fixture = Fixture::new();
            fixture.write("road.x", &plate(3.0, "0.3;0.3;0.3;1"));
            fixture.write("paint.x", &plate(0.5, "1;0;0;1"));
            fixture.write(
                "road.sco",
                "[rendertype]\nsurface\n[surface]\n[mesh]\nroad.x\n",
            );
            fixture.write(
                "paint.sco",
                &format!("[rendertype]\n{queue}\n[LOD]\n0.1\n[mesh]\npaint.x\n[matl]\nwhite.png\n0\n[matl_alpha]\n{alpha}\n[LOD]\n0\n[mesh]\npaint.x\n[matl]\nwhite.png\n0\n[matl_alpha]\n{alpha}\n"),
            );
            fixture.write("tile_0_0.map", "[version]\n14\n[object]\n0\nroad.sco\n10\n100\n100\n2\n0\n0\n0\n0\n[object]\n0\npaint.sco\n11\n100\n100\n2.001\n0\n0\n0\n0\n");
            let world = fixture.world();
            let tile = fixture.prepare(&world);
            let mut scene = renderer.new_scene();
            world.upload_tile(&renderer, &mut scene, tile, &mut LoadStats::default());
            let paints: Vec<_> = scene
                .instances
                .iter()
                .enumerate()
                .filter(|(_, i)| (i.origin.z - 2.001).abs() < 1e-6)
                .map(|(id, i)| {
                    assert!(!i.surface);
                    assert!(i.decal && i.surface_bias);
                    assert_eq!(i.render_phase, phase);
                    id
                })
                .collect();
            assert_eq!(
                paints.len(),
                2,
                "both paint LODs retain their queue and bias"
            );
            let rgba = renderer
                .render_to_image(&mut scene, 128, 128, &camera, &lighting)
                .unwrap();
            let c = &rgba[(64 * 128 + 64) * 4..(64 * 128 + 64) * 4 + 3];
            assert!(
                c[0] > c[1] + 30,
                "queue {queue}, alpha {alpha}: the marking stays above the road: {c:?}"
            );
            // The same geometry without paint compensation reproduces the regression.
            for id in paints {
                scene.instances[id].decal = false;
                scene.instances[id].surface_bias = false;
            }
            let rgba = renderer
                .render_to_image(&mut scene, 128, 128, &camera, &lighting)
                .unwrap();
            let c = &rgba[(64 * 128 + 64) * 4..(64 * 128 + 64) * 4 + 3];
            assert!(c[0].abs_diff(c[1]) < 5, "queue {queue}, alpha {alpha}: without paint compensation the road hides the marking: {c:?}");
        }
    }
}

#[test]
fn numeric_queue_poles_register_collision_but_explicit_surfaces_do_not() {
    let fixture = Fixture::new();
    let world = fixture.world();
    let _ = fixture.prepare(&world);
    let states = world.tile_state.lock();
    let mut ids: Vec<_> = states[&(0, 0)]
        .mesh_obstacles
        .iter()
        .map(|o| o.id)
        .collect();
    ids.sort_unstable();
    assert_eq!(ids, [2, 4], "the street light and normal sign post collide; the explicit surface keeps its existing treatment");
}

#[test]
fn an_after_normal_canopy_is_not_registered_as_ground() {
    let fixture = Fixture::new();
    let world = fixture.world();
    let _ = fixture.prepare(&world);
    let surfaces = world.surfaces.read();
    let surface = &surfaces[&(0, 0)];
    assert_eq!(
        surface.sample(132.0, 107.0),
        None,
        "an elevated ordinary prop must not become ground"
    );
    assert_eq!(
        surface.sample(152.0, 107.0),
        Some(2.0),
        "the same mesh explicitly marked [surface] remains ground"
    );
}

#[test]
#[ignore = "requires a graphics adapter; exercises production scenery upload and depth"]
fn a_sign_in_front_of_an_after_normal_pole_occludes_it() {
    let fixture = Fixture::new();
    let world = fixture.world();
    let tile = fixture.prepare(&world);
    let instance = crate::graphics_instance();
    let mut renderer = pollster::block_on(Renderer::new_with(
        &instance,
        None,
        Some(wgpu::TextureFormat::Rgba8UnormSrgb),
        omsi_render::RenderOptions {
            msaa: 1,
            ssao: false,
            shadow_size: 1024,
            fxaa: false,
            render_scale: 1.0,
            ..Default::default()
        },
    ))
    .expect("test renderer");
    let mut scene = renderer.new_scene();
    world.upload_tile(&renderer, &mut scene, tile, &mut LoadStats::default());
    let lamp: Vec<_> = scene
        .instances
        .iter()
        .filter(|i| i.origin.x == 100.0 && i.origin.y > 105.0)
        .collect();
    assert_eq!(lamp.len(), 2, "both authored pole LODs are uploaded");
    for pole in lamp {
        assert_eq!(pole.render_phase, RenderPhase::AfterNormal);
        assert!(!pole.surface && !pole.surface_bias && !pole.decal);
        assert!(pole.casts_shadow);
        if pole.lod.0 > 0.0 {
            assert!(
                pole.omsi_caster,
                "the main mesh retains its authored [shadow]"
            );
        }
    }
    let post: Vec<_> = scene
        .instances
        .iter()
        .filter(|i| i.origin.x == 80.0)
        .collect();
    assert_eq!(post.len(), 2);
    assert!(post
        .iter()
        .all(|i| i.render_phase == RenderPhase::Normal && !i.surface && !i.surface_bias));
    let surface = scene
        .instances
        .iter()
        .find(|i| i.origin.x == 120.0)
        .unwrap();
    assert!(surface.surface && surface.surface_bias && surface.decal);
    assert_eq!(surface.render_phase, RenderPhase::AfterNormal);
    let camera = omsi_render::Camera {
        position: DVec3::new(100.0, 100.0, 2.0),
        yaw: 0.0,
        pitch: 0.0,
        roll: 0.0,
        fov_deg: 30.0,
        near: 0.1,
        far: 100.0,
    };
    let lighting = omsi_render::Lighting {
        shadows: false,
        fog_density: 0.0,
        ..Default::default()
    };
    let rgba = renderer
        .render_to_image(&mut scene, 64, 64, &camera, &lighting)
        .unwrap();
    let center = &rgba[(32 * 64 + 32) * 4..(32 * 64 + 32) * 4 + 3];
    assert!(
        center[0] > center[1] + 20,
        "the red sign, 4 mm ahead, hides the green pole: {center:?}"
    );
}

// These fixtures are independently authored primitives, not redistributed game content.
#[test]
fn staged_sign_and_pole_keep_their_authored_placement_and_collision() {
    let fixture = Fixture::new();
    let world = fixture.world();
    let lamp = world.object_type("lamp.sco").unwrap();
    assert_eq!(
        lamp.sco.render_type,
        omsi_scenery::sco::RenderType::AfterNormal
    );
    assert!(!lamp.paint && !scenery_ground_surface(&lamp.sco));
    assert!(lamp.collision.is_some() && lamp.sco.crash_mode_pole.is_some());
    let post = world.object_type("post.sco").unwrap();
    assert_eq!(post.sco.render_type, omsi_scenery::sco::RenderType::Normal);
    assert!(!post.paint && !scenery_ground_surface(&post.sco));
    let prepared = fixture.prepare(&world);
    let sign = prepared.objects.iter().find(|o| o.map_id == 1).unwrap();
    let pole = prepared.objects.iter().find(|o| o.map_id == 2).unwrap();
    assert!((sign.pos - DVec3::new(100.0, 105.0, 2.0)).length() < 1e-6);
    assert!((pole.pos - DVec3::new(100.0, 105.004, 2.0)).length() < 1e-6);
    assert!(world.tile_state.lock()[&(0, 0)]
        .mesh_obstacles
        .iter()
        .any(|o| o.id == 2));
}

#[test]
fn loaded_numeric_markings_are_paint_but_upright_scenery_is_not() {
    for (queue, phase) in [
        ("1", omsi_scenery::sco::RenderType::BeforeNormal),
        ("3", omsi_scenery::sco::RenderType::AfterNormal),
        ("4", omsi_scenery::sco::RenderType::AfterVehicles),
    ] {
        let fixture = Fixture::new();
        fixture.write(
            "marking.sco",
            &format!("[rendertype]\n{queue}\n[mesh]\ncanopy.x\n"),
        );
        fixture.write(
            "marking.cfg",
            &format!("[rendertype]\n{queue}\n[mesh]\ncanopy.x\n"),
        );
        fixture.write("inherited_marking.sco", "[model]\nmarking.cfg\n");
        fixture.write(
            "ground.sco",
            &format!("[surface]\n[rendertype]\n{queue}\n[mesh]\ncanopy.x\n"),
        );
        fixture.write(
            "upright.sco",
            &format!("[rendertype]\n{queue}\n[collision_mesh]\npole.x\n[mesh]\npole.x\n[shadow]\n"),
        );
        let world = fixture.world();
        for path in ["marking.sco", "inherited_marking.sco"] {
            let marking = world.object_type(path).unwrap();
            assert_eq!(marking.sco.render_type, phase);
            assert!(!scenery_ground_surface(&marking.sco));
            assert!(
                marking.paint,
                "queue {queue}: {path} retains paint treatment"
            );
        }
        let ground = world.object_type("ground.sco").unwrap();
        assert!(scenery_ground_surface(&ground.sco) && !ground.paint);
        let upright = world.object_type("upright.sco").unwrap();
        assert_eq!(upright.sco.render_type, phase);
        assert!(!upright.paint && !scenery_ground_surface(&upright.sco));
        assert!(upright.collision.is_some());
        assert_eq!(upright.mesh_casts, [true]);
    }
}
