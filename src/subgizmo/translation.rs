use egui::Ui;
use glam::DVec3;

use crate::math::{intersect_plane, ray_to_ray, round_to_interval};

use crate::subgizmo::common::{
    draw_arrow, draw_plane, pick_arrow, pick_plane, plane_binormal, plane_global_origin,
    plane_tangent,
};
use crate::subgizmo::{SubGizmo, SubGizmoConfig, SubGizmoState, TransformKind};
use crate::{GizmoMode, GizmoResult, Ray};

pub(crate) type TranslationSubGizmo = SubGizmoConfig<TranslationState>;

impl SubGizmo for TranslationSubGizmo {
    fn pick(&mut self, ui: &Ui, ray: Ray) -> Option<f64> {
        let pick_result = match self.transform_kind {
            TransformKind::Axis => pick_arrow(self, ray),
            TransformKind::Plane => pick_plane(self, ray),
        };

        self.opacity = pick_result.visibility as _;

        self.update_state_with(ui, |state: &mut TranslationState| {
            state.start_point = pick_result.subgizmo_point;
            state.last_point = pick_result.subgizmo_point;
            state.current_delta = DVec3::ZERO;
        });

        if pick_result.picked {
            Some(pick_result.t)
        } else {
            None
        }
    }

    fn update(&mut self, ui: &Ui, ray: Ray) -> Option<GizmoResult> {
        let state = self.state(ui);

        let mut new_point = if self.transform_kind == TransformKind::Axis {
            point_on_axis(self, ray)
        } else {
            point_on_plane(self.normal(), plane_global_origin(self), ray)?
        };

        let mut new_delta = new_point - state.start_point;

        if self.config.snapping {
            new_delta = if self.transform_kind == TransformKind::Axis {
                snap_translation_vector(self, new_delta)
            } else {
                snap_translation_plane(self, new_delta)
            };
            new_point = state.start_point + new_delta;
        }

        self.update_state_with(ui, |state: &mut TranslationState| {
            state.last_point = new_point;
            state.current_delta = new_delta;
        });

        let new_translation = self.config.translation + new_point - state.last_point;

        Some(GizmoResult {
            scale: self.config.scale.as_vec3().into(),
            rotation: self.config.rotation.as_f32().into(),
            translation: new_translation.as_vec3().into(),
            mode: GizmoMode::Translate,
            value: state.current_delta.as_vec3().to_array(),
        })
    }

    fn draw(&self, ui: &Ui) {
        match self.transform_kind {
            TransformKind::Axis => draw_arrow(self, ui),
            TransformKind::Plane => draw_plane(self, ui),
        }
    }
}

#[derive(Default, Debug, Copy, Clone)]
pub(crate) struct TranslationState {
    start_point: DVec3,
    last_point: DVec3,
    current_delta: DVec3,
}

impl SubGizmoState for TranslationState {}

/// Finds the nearest point on line that points in translation subgizmo direction
fn point_on_axis(subgizmo: &SubGizmoConfig<TranslationState>, ray: Ray) -> DVec3 {
    let origin = subgizmo.config.translation;
    let direction = subgizmo.normal();

    let (_ray_t, subgizmo_t) = ray_to_ray(ray.origin, ray.direction, origin, direction);

    origin + direction * subgizmo_t
}

fn point_on_plane(plane_normal: DVec3, plane_origin: DVec3, ray: Ray) -> Option<DVec3> {
    let mut t = 0.0;
    if !intersect_plane(
        plane_normal,
        plane_origin,
        ray.origin,
        ray.direction,
        &mut t,
    ) {
        None
    } else {
        Some(ray.origin + ray.direction * t)
    }
}

fn snap_translation_vector(subgizmo: &SubGizmoConfig<TranslationState>, new_delta: DVec3) -> DVec3 {
    let delta_length = new_delta.length();
    if delta_length > 1e-5 {
        new_delta / delta_length
            * round_to_interval(delta_length, subgizmo.config.snap_distance as f64)
    } else {
        new_delta
    }
}

fn snap_translation_plane(subgizmo: &SubGizmoConfig<TranslationState>, new_delta: DVec3) -> DVec3 {
    let mut binormal = plane_binormal(subgizmo.direction);
    let mut tangent = plane_tangent(subgizmo.direction);
    if subgizmo.config.local_space() {
        binormal = subgizmo.config.rotation * binormal;
        tangent = subgizmo.config.rotation * tangent;
    }
    let cb = new_delta.cross(-binormal);
    let ct = new_delta.cross(tangent);
    let lb = cb.length();
    let lt = ct.length();
    let n = subgizmo.normal();

    if lb > 1e-5 && lt > 1e-5 {
        binormal * round_to_interval(lt, subgizmo.config.snap_distance as f64) * (ct / lt).dot(n)
            + tangent
                * round_to_interval(lb, subgizmo.config.snap_distance as f64)
                * (cb / lb).dot(n)
    } else {
        new_delta
    }
}
