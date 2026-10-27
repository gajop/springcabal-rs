//! Particle births in the original LUPS draw order and float32 equations.
use super::{Effect, Field, Particle};

pub fn emit_effect(
    field: &mut Field,
    effect: &Effect,
    position: [f32; 3],
    random: &mut impl FnMut() -> f32,
) {
    let maximum_life = effect.life + effect.life_spread;
    let force = effect.force.map(|value| value * maximum_life);
    field.emit(
        (0..effect.count).map(|_| particle(effect, position, force, random)),
        maximum_life + effect.delay_spread,
    );
}

const DEGREES_TO_RADIANS: f32 = core::f32::consts::PI / 180.0;
const MINIMUM_LIFE: f32 = 1.0;

fn particle(
    effect: &Effect,
    position: [f32; 3],
    force: [f32; 3],
    random: &mut impl FnMut() -> f32,
) -> Particle {
    let azimuth = random() * core::f32::consts::TAU;
    let cone = random() * effect.emit_rotation_spread * DEGREES_TO_RADIANS;
    let life = effect.life + random() * effect.life_spread;
    let travel = (effect.speed + random() * effect.speed_spread) * life;
    let delay = random() * effect.delay_spread;
    let size = effect.size + random() * effect.size_spread;
    let (rotation, rotation_end) = if effect.distortion {
        (0.0, 0.0)
    } else {
        (
            random() * effect.rotation_spread,
            (effect.rotation_speed + random() * effect.rotation_speed_spread) * life,
        )
    };
    let (disc_angle, disc) = if effect.start_radius > 0.0 {
        (
            random() * core::f32::consts::TAU,
            random() * effect.start_radius,
        )
    } else {
        (0.0, 0.0)
    };
    let direction = emit_direction(effect.emit_vector, cone, azimuth, travel);
    Particle {
        x: position[0] + disc * disc_angle.sin(),
        y: position[1],
        z: position[2] + disc * disc_angle.cos(),
        direction_x: direction[0],
        direction_y: direction[1],
        direction_z: direction[2],
        force_x: force[0],
        force_y: force[1],
        force_z: force[2],
        age: 0.0,
        birth: 0.0,
        death: 0.0,
        effect: 0,
        delay,
        life: life.max(MINIMUM_LIFE),
        size,
        size_end: size + effect.size_growth * life,
        rotation,
        rotation_end,
        speed_exp: effect.speed_exp,
        size_exp: effect.size_exp,
        rotation_exp: effect.rotation_exp,
        animation_x: if effect.distortion {
            (random() * 2.0 - 1.0).floor()
        } else {
            0.0
        },
        animation_y: if effect.distortion {
            (random() * 2.0 - 1.0).floor()
        } else {
            0.0
        },
        distortion: effect.distortion,
        distortion_strength: effect.distortion_strength,
        distortion_scale: effect.distortion_scale,
        distortion_heat: effect.distortion_heat,
        colormap: effect.colormap,
        palette: 0,
        serial: 0,
        texture: effect.texture,
    }
}

fn emit(vector: [f32; 3], local: [f32; 3]) -> [f32; 3] {
    let [x, y, z] = vector;
    let matrix = [
        x * x,
        x * y - z,
        x * z + y,
        x * y + z,
        y * y,
        y * z - x,
        x * z - y,
        y * z + x,
        z * z,
    ];
    [
        matrix[0] * local[0] + matrix[1] * local[1] + matrix[2] * local[2],
        matrix[3] * local[0] + matrix[4] * local[1] + matrix[5] * local[2],
        matrix[6] * local[0] + matrix[7] * local[1] + matrix[8] * local[2],
    ]
}

fn emit_direction(vector: [f32; 3], cone: f32, azimuth: f32, distance: f32) -> [f32; 3] {
    let up = vector;
    let right = [up[1], up[2], -up[0]];
    let right = cross(up, right);
    let forward = cross(up, right);
    let (sin_cone, cos_cone) = cone.sin_cos();
    let (sin_azimuth, cos_azimuth) = azimuth.sin_cos();
    let a = cos_cone;
    let b = cos_azimuth * sin_cone;
    let c = sin_azimuth * sin_cone;
    let local = [
        (up[0] * a - right[0] * b + forward[0] * c) * distance,
        (up[1] * a - right[1] * b + forward[1] * c) * distance,
        (up[2] * a - right[2] * b + forward[2] * c) * distance,
    ];
    emit(vector, local)
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
