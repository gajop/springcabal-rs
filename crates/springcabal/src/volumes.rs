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
    pub fn draw_ground_circle<E: spring::capabilities::Local>(
        &mut self,
        spring: &spring::Spring<'_, E>,
        x: f32,
        z: f32,
        radius: f32,
        color: [f32; 4],
    ) {
        let Some(list) = self.ensure_cylinder_list(spring) else {
            return;
        };
        let _ = spring.gfx().color(color[0], color[1], color[2], color[3]);
        self.draw_volume(spring, list, x, z, radius);
    }

    fn ensure_cylinder_list<E: spring::capabilities::Local>(
        &mut self,
        spring: &spring::Spring<'_, E>,
    ) -> Option<u32> {
        if self.list != 0 {
            return Some(self.list);
        }

        let extremes = spring.terrain().get_ground_extremes().ok()?;
        let average_ground_height = (extremes.init_min_height + extremes.init_max_height) / 2.0;
        let shape_height =
            HEIGHT_MARGIN + (extremes.init_max_height - extremes.init_min_height) + HEIGHT_MARGIN;
        let list = spring
            .gfx()
            .create_list(|spring| {
                let _ = spring
                    .gfx()
                    .begin_end(gl::TRIANGLES, |spring| emit_cylinder_vertices(spring));
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

    fn draw_volume<E: spring::capabilities::Render>(
        &self,
        spring: &spring::Spring<'_, E>,
        list: u32,
        x: f32,
        z: f32,
        radius: f32,
    ) {
        let _ = spring.gfx().depth_mask(false);
        let _ = spring.gfx().depth_clamp(true);
        let _ = spring.gfx().stencil_test(true);

        let _ = spring.gfx().culling(false);
        let _ = spring.gfx().depth_test(spring::gfx::GfxDepthTestOptions {
            enable: true,
            set_func: false,
            func: 0,
        });
        let _ = spring.gfx().color_mask(spring::gfx::GfxColorMaskOptions {
            red: false,
            green: false,
            blue: false,
            alpha: false,
        });
        let _ = spring.gfx().stencil_op(gl::KEEP, gl::INCR, gl::KEEP);
        let _ = spring.gfx().stencil_mask(STENCIL_MASK);
        let _ = spring.gfx().stencil_func(gl::ALWAYS, 0, 0);
        self.draw_positioned_list(spring, list, x, z, radius);

        let _ = spring.gfx().culling(true);
        let _ = spring.gfx().depth_test(spring::gfx::GfxDepthTestOptions {
            enable: false,
            set_func: false,
            func: 0,
        });
        let _ = spring.gfx().color_mask(spring::gfx::GfxColorMaskOptions {
            red: true,
            green: true,
            blue: true,
            alpha: true,
        });
        let _ = spring.gfx().stencil_op(gl::ZERO, gl::ZERO, gl::ZERO);
        let _ = spring.gfx().stencil_mask(STENCIL_MASK);
        let _ = spring.gfx().stencil_func(gl::NOTEQUAL, 0, 1);
        self.draw_positioned_list(spring, list, x, z, radius);

        let _ = spring.gfx().depth_clamp(false);
        let _ = spring.gfx().stencil_test(false);
        let _ = spring.gfx().depth_test(spring::gfx::GfxDepthTestOptions {
            enable: true,
            set_func: false,
            func: 0,
        });
        let _ = spring.gfx().culling(false);
    }

    fn draw_positioned_list<E: spring::capabilities::Render>(
        &self,
        spring: &spring::Spring<'_, E>,
        list: u32,
        x: f32,
        z: f32,
        radius: f32,
    ) {
        let _ = spring.gfx().push_pop_matrix(|spring| {
            let _ = spring.gfx().translate(x, self.average_ground_height, z);
            let _ = spring.gfx().scale(radius, self.shape_height, radius);
            let _ = spring.gfx().call_list(list);
        });
    }
}

fn emit_cylinder_vertices<E: spring::capabilities::Render>(spring: &spring::Spring<'_, E>) {
    for segment in 0..CYLINDER_SEGMENTS {
        let next = (segment + 1) % CYLINDER_SEGMENTS;
        let (x0, z0) = unit_circle(segment);
        let (x1, z1) = unit_circle(next);

        triangle(spring, [0.0, 0.5, 0.0], [x0, 0.5, z0], [x1, 0.5, z1]);
        triangle(spring, [0.0, -0.5, 0.0], [x1, -0.5, z1], [x0, -0.5, z0]);

        triangle(spring, [x0, 0.5, z0], [x0, -0.5, z0], [x1, 0.5, z1]);
        triangle(spring, [x1, 0.5, z1], [x0, -0.5, z0], [x1, -0.5, z1]);
    }
}

fn unit_circle(segment: usize) -> (f32, f32) {
    let angle = core::f32::consts::TAU * segment as f32 / CYLINDER_SEGMENTS as f32;
    (angle.sin(), angle.cos())
}

fn triangle<E: spring::capabilities::Render>(
    spring: &spring::Spring<'_, E>,
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
) {
    vertex(spring, a);
    vertex(spring, b);
    vertex(spring, c);
}

fn vertex<E: spring::capabilities::Render>(spring: &spring::Spring<'_, E>, position: [f32; 3]) {
    let _ = spring
        .gfx()
        .vertex(position[0], position[1], position[2], 1.0, 3);
}
