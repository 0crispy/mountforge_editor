use std::path::Path;

use bevy::{
    asset::LoadedFolder,
    prelude::*,
};
use bevy_egui::{
    egui::{
        Align2, Button, Color32, DragValue, ImageButton, Pos2, Slider, Ui,
    },
    EguiContexts, EguiPlugin,
};
use bevy_inspector_egui::egui;

use crate::{
    serialize::Serializer,
    terrain::{LODLevel, MasterTerrain},
    VERSION,
};

pub struct TerrainUiPlugin;
impl Plugin for TerrainUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            EguiPlugin,
            //bevy_inspector_egui::quick::WorldInspectorPlugin::new(),
        ))
        .insert_resource(EditInfo::default())
        .insert_resource(UiHovered(false))
        .add_systems(Startup, load_assets)
        .add_systems(Update, update_egui);
    }
}
#[derive(PartialEq, Clone)]
enum QualityPreset {
    Ultra,
    VeryHigh,
    High,
    Medium,
    Low,
    Potato,
}
impl ToString for QualityPreset {
    fn to_string(&self) -> String {
        match self {
            QualityPreset::Ultra => "Ultra",
            QualityPreset::VeryHigh => "Very high",
            QualityPreset::High => "High",
            QualityPreset::Medium => "Medium",
            QualityPreset::Low => "Low",
            QualityPreset::Potato => "Potato",
        }
        .to_string()
    }
}
impl QualityPreset {
    pub fn to_lod(&self) -> Vec<LODLevel> {
        match self {
            QualityPreset::Ultra => {
                vec![LODLevel::new(0, f32::MAX)]
            }
            QualityPreset::VeryHigh => {
                vec![
                    LODLevel::new(0, 200.0),
                    LODLevel::new(1, 1000.0),
                    LODLevel::new(2, 4000.0),
                    LODLevel::new(3, f32::MAX),
                ]
            }
            QualityPreset::High => {
                vec![
                    LODLevel::new(0, 200.0),
                    LODLevel::new(1, 500.0),
                    LODLevel::new(2, 2000.0),
                    LODLevel::new(4, f32::MAX),
                ]
            }
            QualityPreset::Medium => {
                vec![
                    LODLevel::new(0, 200.0),
                    LODLevel::new(1, 500.0),
                    LODLevel::new(4, f32::MAX),
                ]
            }
            QualityPreset::Low => {
                vec![
                    LODLevel::new(1, 200.0),
                    LODLevel::new(2, 500.0),
                    LODLevel::new(4, f32::MAX),
                ]
            }
            QualityPreset::Potato => {
                vec![LODLevel::new(3, f32::MAX)]
            }
        }
    }
}
pub struct NewTerrain {
    active: bool,

    chunk_size: usize,
    texture_size: usize,
    quality: QualityPreset,
}
impl Default for NewTerrain {
    fn default() -> Self {
        Self {
            active: true,

            chunk_size: 128,
            texture_size: 1024,
            quality: QualityPreset::High,
        }
    }
}
#[derive(Resource)]
pub struct EditInfo {
    pub new_terrain: NewTerrain,

    pub debug_active: bool,
    pub edit_mode: EditMode,

    pub brushes: Handle<LoadedFolder>,
    pub textures: Handle<LoadedFolder>,
    pub models: Handle<LoadedFolder>,

    pub sculpt_info: SculptInfo,
    pub edit_chunks_info: EditChunksInfo,
    pub draw_info: DrawInfo,
    pub details_info: DetailsInfo,
}
impl Default for EditInfo {
    fn default() -> Self {
        Self {
            new_terrain: NewTerrain::default(),

            debug_active: true,
            edit_mode: EditMode::EditChunks,

            brushes: Handle::default(),
            textures: Handle::default(),
            models: Handle::default(),

            sculpt_info: SculptInfo::default(),
            edit_chunks_info: EditChunksInfo::default(),
            draw_info: DrawInfo::default(),
            details_info: DetailsInfo::default(),
        }
    }
}
#[derive(PartialEq)]
pub enum SculptType {
    RaiseLower,
    SetHeight,
    Smooth,
}
impl ToString for SculptType {
    fn to_string(&self) -> String {
        match self {
            SculptType::RaiseLower => "Raise/lower terrain",
            SculptType::SetHeight => "Set terrain height",
            SculptType::Smooth => "Smooth terrain",
        }
        .to_string()
    }
}
impl Default for SculptType {
    fn default() -> Self {
        Self::RaiseLower
    }
}
pub struct DetailsInfo {
    pub selected_detail_name: String,
    pub selected_detail: Handle<Scene>,
    pub brush_info: BrushInfo,
}
impl Default for DetailsInfo {
    fn default() -> Self {
        Self {
            selected_detail_name: String::new(),
            selected_detail: Handle::default(),
            brush_info: BrushInfo::default(),
        }
    }
}
pub struct DrawInfo {
    pub draw_texture_info: DrawTextureInfo,
    pub brush_info: BrushInfo,
}
impl Default for DrawInfo {
    fn default() -> Self {
        Self {
            draw_texture_info: DrawTextureInfo::default(),
            brush_info: BrushInfo::default(),
        }
    }
}
pub struct SculptInfo {
    pub sculpt_type: SculptType,

    pub set_height: f32,
    pub auto_height: bool,

    pub brush_info: BrushInfo,
}
impl Default for SculptInfo {
    fn default() -> Self {
        Self {
            set_height: 100.0,
            auto_height: false,
            sculpt_type: SculptType::default(),
            brush_info: BrushInfo::default(),
        }
    }
}
#[derive(PartialEq, Clone)]
pub enum EditChunksAction {
    Remove,
    Add,
}
#[derive(PartialEq, Clone)]
pub struct EditChunksInfo {
    pub action_type: EditChunksAction,
    pub red_chunk: Option<IVec2>,
}
impl Default for EditChunksInfo {
    fn default() -> Self {
        Self {
            action_type: EditChunksAction::Add,
            red_chunk: None,
        }
    }
}
#[derive(PartialEq, Clone)]
pub enum EditMode {
    View,
    EditChunks,
    Sculpt,
    Draw,
    EditDetails,
}
pub const EDIT_MODES: [EditMode; 5] = [
    EditMode::View,
    EditMode::EditChunks,
    EditMode::Sculpt,
    EditMode::Draw,
    EditMode::EditDetails,
];
impl ToString for EditMode {
    fn to_string(&self) -> String {
        match self {
            EditMode::View => "View",
            EditMode::EditChunks => "Edit chunks",
            EditMode::Sculpt => "Sculpt terrain",
            EditMode::Draw => "Draw textures",
            EditMode::EditDetails => "Edit details",
        }
        .to_string()
    }
}
fn load_assets(asset_server: Res<AssetServer>, mut edit_info: ResMut<EditInfo>) {
    edit_info.brushes = asset_server.load_folder("brushes");
    edit_info.textures = asset_server.load_folder("textures");
    edit_info.models = asset_server.load_folder("models");
}
#[derive(Resource)]
pub struct UiHovered(pub bool);

pub fn update_egui(
    mut contexts: EguiContexts,
    time: Res<Time>,
    mut edit_info: ResMut<EditInfo>,
    images: Res<Assets<Image>>,
    loaded_folders: Res<Assets<LoadedFolder>>,
    asset_server: Res<AssetServer>,
    mut ui_hovered: ResMut<UiHovered>,
    q_windows: Query<&Window>,
    mut serializer: ResMut<Serializer>,
    mut master_terrain: ResMut<MasterTerrain>,
) {
    let mouse = q_windows.single().cursor_position().unwrap_or(Vec2::ZERO);
    let mouse = Pos2::new(mouse.x, mouse.y);
    ui_hovered.0 = false;
    let response = egui::TopBottomPanel::top("Top")
        .show(contexts.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                if ui.button(format!("Mountforge v. {}", VERSION)).clicked() {
                    edit_info.new_terrain.active = true;
                }
                if ui.button("Save as").clicked() {
                    let path = std::env::current_dir().unwrap();
                    let res = rfd::FileDialog::new().set_directory(path).save_file();
                    if let Some(path) = res {
                        //do sth with path
                        serializer.serialize(path);
                    }
                }
                if ui.button("Debug").clicked() {
                    edit_info.debug_active = true;
                }
            });
        })
        .response;
    ui_hovered.0 = response.rect.contains(mouse) || ui_hovered.0;
    if let Some(response) = egui::Window::new("Debug")
        .open(&mut edit_info.debug_active)
        .show(contexts.ctx_mut(), |ui| {
            ui.label(format!("FPS: {:.1}", 1.0 / time.delta_seconds()));
        })
    {
        ui_hovered.0 = response.response.rect.contains(mouse) || ui_hovered.0;
    }
    if edit_info.new_terrain.active {
        let response = egui::Window::new("Welcome to Mountforge!")
            .anchor(Align2::CENTER_CENTER, bevy_egui::egui::Vec2::ZERO)
            .collapsible(false)
            .show(contexts.ctx_mut(), |ui| {
                ui.heading("Controls:");
                ui.label("Pan the camera using the middle mouse button.");
                ui.label("Orbit the camera using the right mouse button.");
                ui.label("Interact with the left mouse button.");
                ui.label("Lower the terrain by holding the control key and pressing the left mouse button.");
                ui.separator();
                ui.heading("Load:");
                if ui.button("Load from file").clicked() {
                    let path = std::env::current_dir().unwrap();
                    let res = rfd::FileDialog::new()
                        .set_directory(path)
                        .add_filter("mf", &["mf"])
                        .pick_file();
                    if let Some(path) = res {
                        serializer.deserialize(path);
                        edit_info.new_terrain.active = false;
                    }
                }
                let mut is_invalid = false;
                ui.heading("New:");
                ui.label("Chunk size: (recommended 128)");
                ui.add(DragValue::new(&mut edit_info.new_terrain.chunk_size));
                if edit_info.new_terrain.chunk_size % 16 != 0 {
                    ui.colored_label(Color32::RED, "Chunk size must be divisible by 16!");
                    is_invalid = true;
                }
                ui.label("Texture size: (recommended 1024)");
                ui.add(DragValue::new(&mut edit_info.new_terrain.texture_size));
                if edit_info.new_terrain.texture_size % edit_info.new_terrain.chunk_size != 0 {
                    ui.colored_label(
                        Color32::RED,
                        "Texture size must be divisible by the chunk size!",
                    );
                    is_invalid = true;
                }
                ui.label("Quality preset:");
                egui::ComboBox::from_label("")
                    .selected_text(format!("{}", edit_info.new_terrain.quality.to_string()))
                    .show_ui(ui, |ui| {
                        for quality_preset in [
                            QualityPreset::Ultra,
                            QualityPreset::VeryHigh,
                            QualityPreset::High,
                            QualityPreset::Medium,
                            QualityPreset::Low,
                            QualityPreset::Potato,
                        ] {
                            ui.selectable_value(
                                &mut edit_info.new_terrain.quality,
                                quality_preset.clone(),
                                quality_preset.to_string(),
                            );
                        }
                    });
                ui.separator();
                if ui
                    .add_enabled(!is_invalid, Button::new("New terrain"))
                    .clicked()
                    && !is_invalid
                {
                    master_terrain.init(
                        edit_info.new_terrain.chunk_size,
                        edit_info.new_terrain.texture_size,
                        edit_info.new_terrain.quality.to_lod(),
                    );
                    edit_info.new_terrain.active = false;
                }
            })
            .unwrap()
            .response;
        ui_hovered.0 = response.rect.contains(mouse) || ui_hovered.0;

        return;
    }
    let response = egui::Window::new("Edit")
        .anchor(Align2::RIGHT_TOP, bevy_egui::egui::Vec2::new(-10.0, 10.0))
        .collapsible(false)
        .show(contexts.ctx_mut(), |ui| {
            for edit_mode in EDIT_MODES {
                ui.selectable_value(
                    &mut edit_info.edit_mode,
                    edit_mode.clone(),
                    edit_mode.to_string(),
                );
            }
        })
        .unwrap()
        .response;
    ui_hovered.0 = response.rect.contains(mouse) || ui_hovered.0;
    let mut brush_texture_ids = Vec::new();
    if let Some(brushes_folder) = loaded_folders.get(&edit_info.brushes) {
        for handle in &brushes_folder.handles {
            if images.get(handle).is_some() {
                if let Some(id) = contexts.image_id(&handle.clone().typed()) {
                    brush_texture_ids.push((id, handle.clone().typed::<Image>()));
                } else {
                    contexts.add_image(handle.clone().typed());
                }
            }
        }
    }
    let mut draw_texture_ids = Vec::new();
    if let Some(textures_folder) = loaded_folders.get(&edit_info.textures) {
        for handle in &textures_folder.handles {
            if images.get(handle).is_some() {
                if let Some(id) = contexts.image_id(&handle.clone().typed()) {
                    draw_texture_ids.push((id, handle.clone().typed::<Image>()));
                } else {
                    contexts.add_image(handle.clone().typed());
                }
            }
        }
    }
    let mut models: Vec<String> = Vec::new();
    if let Some(models_folder) = loaded_folders.get(&edit_info.models) {
        for handle in &models_folder.handles {
            if let Some(path) = asset_server.get_path(handle.id()) {
                let path = path.to_string();
                let path = Path::new(&path);
                models.push(path.file_name().unwrap().to_str().unwrap().to_string());
            }
        }
    }
    let brushes = |ui: &mut Ui, brush_info: &mut BrushInfo| {
        ui.label("Brushes");
        ui.horizontal_wrapped(|ui| {
            for (i, (texture_id, handle)) in brush_texture_ids.into_iter().enumerate() {
                if ui
                    .add(
                        ImageButton::new(egui::load::SizedTexture::new(
                            texture_id,
                            egui::vec2(40., 40.),
                        ))
                        .selected(match &brush_info.selected_brush {
                            Some(brush) => i == brush.id,
                            None => false,
                        }),
                    )
                    .clicked()
                {
                    let should_update = match &brush_info.selected_brush {
                        Some(brush) => i != brush.id,
                        None => true,
                    };
                    if should_update {
                        let image = images.get(handle).unwrap();
                        let mut map = Vec::new();
                        for b in image.data.chunks(4) {
                            map.push(b[0] as f32 / 255.0);
                        }

                        brush_info.selected_brush = Some(Brush {
                            id: i,
                            map,
                            map_size: image.width(),
                            sample_map: Vec::new(),
                            sample_map_size: 0,
                        });
                    }
                }
            }
        });
        ui.label("Strength:");
        ui.add(Slider::new(&mut brush_info.strength, 0.0..=1.0));
        ui.label("Size:");
        ui.add(Slider::new(&mut brush_info.size, 1..=200));
    };
    let response = egui::Window::new(edit_info.edit_mode.to_string())
        .anchor(Align2::RIGHT_TOP, bevy_egui::egui::Vec2::new(-10.0, 165.0))
        .collapsible(false)
        .show(contexts.ctx_mut(), |ui| match &mut edit_info.edit_mode {
            EditMode::View => {}
            EditMode::EditChunks => {
                ui.selectable_value(
                    &mut edit_info.edit_chunks_info.action_type,
                    EditChunksAction::Add,
                    "Add chunks",
                );
                ui.selectable_value(
                    &mut edit_info.edit_chunks_info.action_type,
                    EditChunksAction::Remove,
                    "Remove chunks",
                );
            }

            EditMode::Sculpt => {
                egui::ComboBox::from_label("Select sculpt type")
                    .selected_text(format!("{}", edit_info.sculpt_info.sculpt_type.to_string()))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut edit_info.sculpt_info.sculpt_type,
                            SculptType::RaiseLower,
                            SculptType::RaiseLower.to_string(),
                        );
                        ui.selectable_value(
                            &mut edit_info.sculpt_info.sculpt_type,
                            SculptType::SetHeight,
                            SculptType::SetHeight.to_string(),
                        );
                        ui.selectable_value(
                            &mut edit_info.sculpt_info.sculpt_type,
                            SculptType::Smooth,
                            SculptType::Smooth.to_string(),
                        );
                    });
                brushes(ui, &mut edit_info.sculpt_info.brush_info);
                match edit_info.sculpt_info.sculpt_type {
                    SculptType::RaiseLower => {}
                    SculptType::SetHeight => {
                        ui.label("Height:");
                        ui.horizontal(|ui| {
                            ui.add_enabled(
                                !edit_info.sculpt_info.auto_height,
                                DragValue::new(&mut edit_info.sculpt_info.set_height),
                            );
                            ui.checkbox(&mut edit_info.sculpt_info.auto_height, "Auto");
                        });
                    }
                    SculptType::Smooth => {}
                }
            }
            EditMode::Draw => {
                ui.label("Textures");
                ui.horizontal_wrapped(|ui| {
                    for (i, (texture_id, handle)) in draw_texture_ids.into_iter().enumerate() {
                        if ui
                            .add(
                                ImageButton::new(egui::load::SizedTexture::new(
                                    texture_id,
                                    egui::vec2(40., 40.),
                                ))
                                .selected(
                                    match &edit_info.draw_info.draw_texture_info.selected_texture {
                                        Some(texture) => i == texture.id,
                                        None => false,
                                    },
                                ),
                            )
                            .clicked()
                        {
                            let should_update =
                                match &edit_info.draw_info.draw_texture_info.selected_texture {
                                    Some(texture) => i != texture.id,
                                    None => true,
                                };
                            if should_update {
                                let image = images.get(handle).unwrap();
                                let mut map = Vec::new();
                                for b in image.data.chunks(4) {
                                    map.push(Color::rgba_u8(b[0], b[1], b[2], b[3]));
                                }

                                edit_info.draw_info.draw_texture_info.selected_texture =
                                    Some(DrawTexture {
                                        id: i,
                                        map,
                                        map_size: image.width(),
                                        sample_map: Vec::new(),
                                        sample_map_size: 0,
                                    });
                            }
                        }
                    }
                });
                ui.label("Scale:");
                ui.horizontal(|ui| {
                    ui.add(
                        DragValue::new(&mut edit_info.draw_info.draw_texture_info.scale)
                            .clamp_range(1..=usize::MAX),
                    );
                });
                brushes(ui, &mut edit_info.draw_info.brush_info);
            }
            EditMode::EditDetails => {
                egui::ComboBox::from_label("Models")
                    .selected_text(format!("{}", edit_info.details_info.selected_detail_name))
                    .show_ui(ui, |ui| {
                        for name in models {
                            ui.selectable_value(
                                &mut edit_info.details_info.selected_detail_name,
                                name.to_string(),
                                name.to_string(),
                            );
                        }
                    });
                brushes(ui, &mut edit_info.details_info.brush_info);
            }
        })
        .unwrap()
        .response;
    ui_hovered.0 = response.rect.contains(mouse) || ui_hovered.0;
}
pub struct Brush {
    pub id: usize,
    pub map: Vec<f32>,
    pub map_size: u32,

    pub sample_map: Vec<f32>,
    pub sample_map_size: u32,
}
pub struct BrushInfo {
    pub selected_brush: Option<Brush>,

    pub strength: f32,
    pub size: u32,
}
impl Default for BrushInfo {
    fn default() -> Self {
        Self {
            selected_brush: None,

            strength: 1.0,
            size: 100,
        }
    }
}

pub struct DrawTextureInfo {
    pub selected_texture: Option<DrawTexture>,
    pub scale: u32,
}
impl Default for DrawTextureInfo {
    fn default() -> Self {
        Self {
            selected_texture: None,
            scale: 100,
        }
    }
}
pub struct DrawTexture {
    pub id: usize,

    pub map: Vec<Color>,
    pub map_size: u32,

    pub sample_map: Vec<Color>,
    pub sample_map_size: u32,
}
