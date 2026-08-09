use crate::terrain::TerrainBounds;
use bevy::prelude::*;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, create_terrain_render_assets)
            .add_systems(Update, sync_terrain);
    }
}

#[derive(Component)]
pub struct TerrainRoot;

#[derive(Component)]
struct TerrainSurface;

#[derive(Clone, Copy, Component, Debug, Eq, PartialEq)]
struct RenderedTerrain {
    bounds: TerrainBounds,
}

#[derive(Resource)]
struct TerrainRenderAssets {
    surface_mesh: Handle<Mesh>,
    surface_material: Handle<StandardMaterial>,
}

fn create_terrain_render_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(TerrainRenderAssets {
        surface_mesh: meshes.add(Plane3d::default().mesh().size(1.0, 1.0)),
        surface_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.32, 0.20),
            perceptual_roughness: 0.95,
            ..default()
        }),
    });
}

fn sync_terrain(
    mut commands: Commands,
    bounds: Option<Res<TerrainBounds>>,
    assets: Res<TerrainRenderAssets>,
    roots: Query<(Entity, &RenderedTerrain), With<TerrainRoot>>,
) {
    let Some(bounds) = bounds else {
        return;
    };

    let rendered = RenderedTerrain { bounds: *bounds };

    if roots.iter().count() == 1
        && roots
            .iter()
            .next()
            .is_some_and(|(_, current)| *current == rendered)
    {
        return;
    }

    for (entity, _) in &roots {
        commands.entity(entity).despawn();
    }

    spawn_terrain(&mut commands, &assets, rendered);
}

fn spawn_terrain(commands: &mut Commands, assets: &TerrainRenderAssets, rendered: RenderedTerrain) {
    let geometry = TerrainGeometry::new(rendered.bounds);
    let root = commands
        .spawn((
            Name::new("Terrain"),
            TerrainRoot,
            rendered,
            Transform::default(),
            Visibility::default(),
        ))
        .id();

    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Name::new("Terrain Surface"),
            TerrainSurface,
            Mesh3d(assets.surface_mesh.clone()),
            MeshMaterial3d(assets.surface_material.clone()),
            geometry.surface_transform,
        ));
    });
}

#[derive(Debug, PartialEq)]
struct TerrainGeometry {
    surface_transform: Transform,
}

impl TerrainGeometry {
    fn new(bounds: TerrainBounds) -> Self {
        let world_min = bounds.world_min();
        let world_max = bounds.world_max();
        let center = (world_min + world_max) * 0.5;
        let width = world_max.x - world_min.x;
        let depth = world_max.z - world_min.z;

        Self {
            surface_transform: Transform::from_translation(center)
                .with_scale(Vec3::new(width, 1.0, depth)),
        }
    }
}
