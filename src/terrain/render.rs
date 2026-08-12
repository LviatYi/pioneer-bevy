use crate::terrain::chunk::mvp_surface_chunks;
use crate::terrain::mesh::extract_terrain_mesh;
use crate::terrain::{LandformGenerator, TerrainChunkCache, TerrainChunkSamples, TerrainConfig};
use bevy::prelude::*;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LandformGenerator>()
            .init_resource::<TerrainChunkCache>()
            .init_resource::<TerrainRebuildRequest>()
            .add_systems(Startup, create_terrain_render_assets)
            .add_systems(
                Update,
                (
                    queue_terrain_rebuild,
                    clear_rendered_terrain,
                    rebuild_terrain,
                )
                    .chain(),
            );
    }
}

#[derive(Component)]
pub struct TerrainRoot;

#[derive(Component)]
struct TerrainSurface {
    mesh: Handle<Mesh>,
}

#[derive(Clone, Copy, Component, Debug, Eq, PartialEq)]
struct RenderedTerrain {
    config: TerrainConfig,
}

#[derive(Resource)]
struct TerrainRenderAssets {
    surface_material: Handle<StandardMaterial>,
}

#[derive(Default, Resource)]
struct TerrainRebuildRequest(Option<RenderedTerrain>);

fn create_terrain_render_assets(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(TerrainRenderAssets {
        surface_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.32, 0.20),
            perceptual_roughness: 0.95,
            ..default()
        }),
    });
}

fn queue_terrain_rebuild(
    config: Res<TerrainConfig>,
    landform: Res<LandformGenerator>,
    roots: Query<(Entity, &RenderedTerrain), With<TerrainRoot>>,
    mut request: ResMut<TerrainRebuildRequest>,
) {
    let rendered = RenderedTerrain { config: *config };

    if !config.is_changed()
        && !landform.is_changed()
        && roots.iter().count() == 1
        && roots
            .iter()
            .next()
            .is_some_and(|(_, current)| *current == rendered)
    {
        return;
    }

    request.0 = Some(rendered);
}

fn clear_rendered_terrain(
    mut commands: Commands,
    request: Res<TerrainRebuildRequest>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cache: ResMut<TerrainChunkCache>,
    roots: Query<Entity, With<TerrainRoot>>,
    surfaces: Query<&TerrainSurface>,
) {
    if request.0.is_none() {
        return;
    }

    for surface in &surfaces {
        meshes.remove(&surface.mesh);
    }
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    cache.clear();
}

fn rebuild_terrain(
    mut commands: Commands,
    mut request: ResMut<TerrainRebuildRequest>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cache: ResMut<TerrainChunkCache>,
    assets: Res<TerrainRenderAssets>,
    landform: Res<LandformGenerator>,
) {
    let Some(rendered) = request.0.take() else {
        return;
    };

    spawn_terrain(
        &mut commands,
        &mut meshes,
        &mut cache,
        &assets,
        &landform,
        rendered,
    );
}

fn spawn_terrain(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    cache: &mut TerrainChunkCache,
    assets: &TerrainRenderAssets,
    landform: &LandformGenerator,
    rendered: RenderedTerrain,
) {
    let terrain_origin = rendered.config.world_min();
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
        for (coord, cells) in mvp_surface_chunks(rendered.config) {
            let samples = TerrainChunkSamples::sample(coord, terrain_origin, cells, landform);
            let chunk_origin = samples.origin();
            let mesh_data = match extract_terrain_mesh(&samples) {
                Ok(mesh_data) => mesh_data,
                Err(error) => {
                    warn!(?coord, %error, "failed to generate terrain chunk mesh");
                    cache.insert(samples);
                    continue;
                }
            };
            cache.insert(samples);

            if mesh_data.is_empty() {
                continue;
            }

            let mesh = meshes.add(mesh_data.into_mesh());
            parent.spawn((
                Name::new(format!("Terrain Chunk {:?}", coord.0)),
                TerrainSurface { mesh: mesh.clone() },
                Mesh3d(mesh),
                MeshMaterial3d(assets.surface_material.clone()),
                Transform::from_translation(chunk_origin),
            ));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::TerrainConfig;
    use crate::terrain::landform_algorithm::flat::FlatLandform;

    fn test_app() -> App {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .insert_resource(TerrainConfig::default())
            .add_plugins(TerrainPlugin);
        app
    }

    fn component_count<T: Component>(world: &mut World) -> usize {
        let mut query = world.query::<&T>();
        query.iter(world).count()
    }

    fn single_entity<T: Component>(world: &mut World) -> Entity {
        let mut query = world.query_filtered::<Entity, With<T>>();
        query.single(world).unwrap()
    }

    #[test]
    fn plugin_builds_cached_chunk_meshes_and_replaces_them_without_leaking() {
        let mut app = test_app();
        app.update();

        assert_eq!(app.world().resource::<TerrainChunkCache>().len(), 8);
        assert_eq!(component_count::<TerrainRoot>(app.world_mut()), 1);
        assert_eq!(component_count::<TerrainSurface>(app.world_mut()), 4);
        assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 4);
        let initial_root = single_entity::<TerrainRoot>(app.world_mut());

        app.update();

        assert_eq!(single_entity::<TerrainRoot>(app.world_mut()), initial_root);

        *app.world_mut().resource_mut::<LandformGenerator>() =
            LandformGenerator::new(FlatLandform {
                surface_height: 4.0,
            });
        app.update();

        assert_eq!(app.world().resource::<TerrainChunkCache>().len(), 8);
        assert_eq!(component_count::<TerrainRoot>(app.world_mut()), 1);
        assert_eq!(component_count::<TerrainSurface>(app.world_mut()), 4);
        assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 4);
        assert_ne!(single_entity::<TerrainRoot>(app.world_mut()), initial_root);
    }
}
