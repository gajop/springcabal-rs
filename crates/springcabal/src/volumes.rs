use spring::gl;

const CYLINDER_SEGMENTS: usize = 35;
const HEIGHT_MARGIN: f32 = 2000.0;
const STENCIL_MASK: u32 = 0x11;

#[derive(Default)]
pub struct Volumes {
    list: u32,
    average_ground_height: f32,
    shape_height: f32,
}

impl Volumes {
    pub fn draw_ground_circle(&mut self, x: f32, z: f32, radius: f32, color: [f32; 4]) {
        let Some(list) = self.ensure_cylinder_list() else {
            return;
        };
        let _ = spring::color(color[0], color[1], color[2], color[3]);
        self.draw_volume(list, x, z, radius);
    }

    fn ensure_cylinder_list(&mut self) -> Option<u32> {
        if self.list != 0 {
            return Some(self.list);
        }

        let extremes = spring::get_ground_extremes().ok()?;
        let average_ground_height = (extremes.initial_min + extremes.initial_max) / 2.0;
        let shape_height =
            HEIGHT_MARGIN + (extremes.initial_max - extremes.initial_min) + HEIGHT_MARGIN;
        let list = spring::create_list(|| {
            let _ = spring::begin_end(gl::TRIANGLES, emit_cylinder_vertices);
        })
        .ok()?;
        if list == 0 {
            return None;
        }

        self.list = list;
        self.average_ground_height = average_ground_height;
        self.shape_height = shape_height;
        Some(list)
    }

    fn draw_volume(&self, list: u32, x: f32, z: f32, radius: f32) {
        let _ = spring::depth_mask(false);
        let _ = spring::depth_clamp(true);
        let _ = spring::stencil_test(true);

        let _ = spring::culling(false);
        let _ = spring::depth_test(spring::ui::gfx::GfxDepthTestOptions {
            enable: true,
            set_func: false,
            func: 0,
        });
        let _ = spring::color_mask(spring::ui::gfx::GfxColorMaskOptions {
            red: false,
            green: false,
            blue: false,
            alpha: false,
        });
        let _ = spring::stencil_op(gl::KEEP, gl::INCR, gl::KEEP);
        let _ = spring::stencil_mask(STENCIL_MASK);
        let _ = spring::stencil_func(gl::ALWAYS, 0, 0);
        self.draw_positioned_list(list, x, z, radius);

        let _ = spring::culling(true);
        let _ = spring::depth_test(spring::ui::gfx::GfxDepthTestOptions {
            enable: false,
            set_func: false,
            func: 0,
        });
        let _ = spring::color_mask(spring::ui::gfx::GfxColorMaskOptions {
            red: true,
            green: true,
            blue: true,
            alpha: true,
        });
        let _ = spring::stencil_op(gl::ZERO, gl::ZERO, gl::ZERO);
        let _ = spring::stencil_mask(STENCIL_MASK);
        let _ = spring::stencil_func(gl::NOTEQUAL, 0, 1);
        self.draw_positioned_list(list, x, z, radius);

        let _ = spring::depth_clamp(false);
        let _ = spring::stencil_test(false);
        let _ = spring::depth_test(spring::ui::gfx::GfxDepthTestOptions {
            enable: true,
            set_func: false,
            func: 0,
        });
        let _ = spring::culling(false);
    }

    fn draw_positioned_list(&self, list: u32, x: f32, z: f32, radius: f32) {
        let _ = spring::push_pop_matrix(|| {
            let _ = spring::translate(x, self.average_ground_height, z);
            let _ = spring::scale(radius, self.shape_height, radius);
            let _ = spring::call_list(list);
        });
    }
}

fn emit_cylinder_vertices() {
    for segment in 0..CYLINDER_SEGMENTS {
        let next = (segment + 1) % CYLINDER_SEGMENTS;
        let (x0, z0) = unit_circle(segment);
        let (x1, z1) = unit_circle(next);

        triangle([0.0, 0.5, 0.0], [x0, 0.5, z0], [x1, 0.5, z1]);
        triangle([0.0, -0.5, 0.0], [x1, -0.5, z1], [x0, -0.5, z0]);

        triangle([x0, 0.5, z0], [x0, -0.5, z0], [x1, 0.5, z1]);
        triangle([x1, 0.5, z1], [x0, -0.5, z0], [x1, -0.5, z1]);
    }
}

fn unit_circle(segment: usize) -> (f32, f32) {
    let angle = core::f32::consts::TAU * segment as f32 / CYLINDER_SEGMENTS as f32;
    (angle.sin(), angle.cos())
}

fn triangle(a: [f32; 3], b: [f32; 3], c: [f32; 3]) {
    vertex(a);
    vertex(b);
    vertex(c);
}

fn vertex(position: [f32; 3]) {
    let _ = spring::vertex(position[0], position[1], position[2], 1.0, 3);
}
