use notan::draw::*;
use notan::math::Mat3;
use notan::math::Vec3;
use notan::prelude::*;

const WIDTH: usize = 192;
const HEIGHT: usize = 108 / 2;
const BYTES_LENGTH: usize = WIDTH * HEIGHT * 2;

// The constant 'D' represents the distance between the camera and the projection plane.
const D: f32 = 1.0;

struct Sphere {
    center: Vec3,
    radius: f32,
}

struct Viewport {
    width: f32,
    height: f32,
}

struct Camera {
    position: Vec3,
    rotation: Mat3,
    viewport: Viewport,
    buffer: [[char; WIDTH]; HEIGHT],
}

impl Camera {
    fn get_width(&self) -> usize {
        self.buffer[0].len()
    }

    fn get_height(&self) -> usize {
        self.buffer.len()
    }

    fn camera_pixel_to_viewport_distance(&self, x: f32, y: f32) -> Vec3 {
        Vec3 {
            x: x * self.viewport.width / self.get_width() as f32,
            y: y * self.viewport.height / self.get_height() as f32,
            z: D,
        }
    }
}

#[derive(AppState)]
struct State {
    font: Font,
    camera: Camera,
    spheres: Vec<Sphere>,
    current_bytes: [u8; BYTES_LENGTH],
    previous_bytes: [u8; BYTES_LENGTH],
    buffer: [char; BYTES_LENGTH],
    count: f32,
    dirty: bool,
}

impl State {
    fn is_alive(&self, x: usize, y: usize) -> bool {
        let neighbors = get_neighbors(x as _, y as _);
        let count = neighbors.iter().fold(0, |sum, (x, y)| {
            let idx = index(*x, *y);
            match idx {
                Some(idx) => {
                    let is_red =
                        is_red_color(&self.previous_bytes[idx..idx + 4].try_into().unwrap());
                    if is_red {
                        sum + 1
                    } else {
                        sum
                    }
                }
                _ => sum,
            }
        });

        let was_alive = match index(x as _, y as _) {
            Some(idx) => is_red_color(&self.previous_bytes[idx..idx + 4].try_into().unwrap()),
            _ => false,
        };

        if was_alive {
            count == 2 || count == 3
        } else {
            count == 3
        }
    }

    fn swap_data(&mut self) {
        std::mem::swap(&mut self.current_bytes, &mut self.previous_bytes);
        self.dirty = true;
    }

    fn set_color(&mut self, color: Color, x: usize, y: usize) {
        if let Some(idx) = index(x as _, y as _) {
            self.current_bytes[idx..idx + 4].copy_from_slice(&color.rgba_u8());
        }
    }
}

#[notan_main]
fn main() -> Result<(), String> {
    let width = WIDTH * 6;
    let height = HEIGHT * 16;

    let win_config = WindowConfig::new().set_size(width as _, height as _);

    notan::init_with(setup)
        .initialize(init)
        .add_config(win_config)
        .add_config(DrawConfig)
        .update(update)
        .draw(draw)
        .build()
}

fn setup(gfx: &mut Graphics) -> State {
    let current_bytes = [255; BYTES_LENGTH];
    let previous_bytes = current_bytes;

    let font = gfx
        .create_font(include_bytes!("../assets/fonts/NotoSansMono-Regular.ttf"))
        .unwrap();

    let camera = Camera {
        position: Vec3::default(),
        rotation: Mat3::default(),
        viewport: Viewport {
            width: 1.0,
            height: 1.0,
        },
        buffer: [['-'; WIDTH]; HEIGHT],
    };

    State {
        font,
        camera,
        current_bytes,
        previous_bytes,
        spheres: Vec::new(),
        buffer: [' '; BYTES_LENGTH],
        count: 0.0,
        dirty: false,
    }
}

fn init(state: &mut State) {
    state.spheres = vec![
        Sphere {
            center: Vec3 {
                x: 0.0,
                y: -1.0,
                z: 3.0,
            },
            radius: 1.0,
        },
        Sphere {
            center: Vec3 {
                x: 2.0,
                y: 0.0,
                z: 4.0,
            },
            radius: 1.0,
        },
        Sphere {
            center: Vec3 {
                x: -2.0,
                y: 0.0,
                z: 4.0,
            },
            radius: 1.0,
        },
        Sphere {
            center: Vec3 {
                x: 0.0,
                y: -5001.0,
                z: 0.0,
            },
            radius: 5000.0,
        },
    ];

    let mut rng = Random::default();
    for _ in 0..500 {
        let x = rng.gen_range(0..WIDTH);
        let y = rng.gen_range(0..HEIGHT);

        let neighbors = get_neighbors(x as _, y as _);
        neighbors.iter().for_each(|(x, y)| {
            let valid_coords = index(*x, *y).is_some();
            if valid_coords {
                state.set_color(Color::RED, *x as _, *y as _);
            }
        });
    }

    state.swap_data();
}

fn ray_intersects_sphere(origin: Vec3, direction: Vec3, sphere: &Sphere) -> (f32, f32) {
    let r = sphere.radius;

    let co = origin - sphere.center;

    let a = direction.dot(direction);
    let b = 2.0 * co.dot(direction);
    let c = co.dot(co) - r * r;

    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return (f32::INFINITY, f32::INFINITY);
    }

    let t1 = (-b + discriminant.sqrt()) / (2.0 * a);
    let t2 = (-b - discriminant.sqrt()) / (2.0 * a);

    (t1, t2)
}

fn compute_lighting(p: Vec3, n: Vec3) -> char {
    let mut i = 0.2;
    let light_pos = Vec3 {
        x: 2.0,
        y: 1.0,
        z: 0.0,
    };

    let l = light_pos - p;

    let n_dot_l = n.dot(l);
    if n_dot_l > 0.0 {
        i += 0.6 * n_dot_l / (n.length() * l.length());
    }

    let scale = [
        '.', ',', ':', ';', '*', '+', 'o', 'x', '%', '&', '#', '$', '@', '9',
    ];
    let index = (i * scale.len() as f32) as usize;
    return scale[index];
}

fn trace_ray(origin: Vec3, direction: Vec3, t_min: f32, t_max: f32, spheres: &[Sphere]) -> char {
    let mut closest_t: f32 = f32::INFINITY;
    let mut closest_sphere: Option<&Sphere> = None;

    for sphere in spheres {
        let (t1, t2) = ray_intersects_sphere(origin, direction, &sphere);

        if t_min < t1 && t1 < t_max && t1 < closest_t {
            closest_t = t1;
            closest_sphere = Some(&sphere);
        }
        if t_min < t2 && t2 < t_max && t2 < closest_t {
            closest_t = t2;
            closest_sphere = Some(&sphere);
        }
    }

    if let Some(s) = closest_sphere {
        let p = origin + closest_t * direction;
        let n = p - s.center;

        return compute_lighting(p, n / n.length());
    } else {
        return ' ';
    }
}

fn update(app: &mut App, state: &mut State) {
    state.count += app.timer.delta_f32();

    if app.keyboard.is_down(KeyCode::W) {
        state.camera.position += state.camera.rotation * Vec3::from_array([0.0, 0.0, 0.05]);
    }
    if app.keyboard.is_down(KeyCode::S) {
        state.camera.position -= state.camera.rotation * Vec3::from_array([0.0, 0.0, 0.05]);
    }
    if app.keyboard.is_down(KeyCode::A) {
        state.camera.position -= state.camera.rotation * Vec3::from_array([0.05, 0.0, 0.0]);
    }
    if app.keyboard.is_down(KeyCode::D) {
        state.camera.position += state.camera.rotation * Vec3::from_array([0.05, 0.0, 0.0]);
    }
    if app.keyboard.is_down(KeyCode::E) {
        state.camera.rotation *= Mat3::from_rotation_y(0.025);
    }
    if app.keyboard.is_down(KeyCode::Q) {
        state.camera.rotation *= Mat3::from_rotation_y(0.025).inverse();
    }

    // update each 100ms
    if state.count >= 0.1 {
        let (width, height): (i32, i32) = (WIDTH as i32, HEIGHT as i32);

        for x in -width / 2..width / 2 {
            for y in -height / 2..height / 2 {
                let d: Vec3 = state.camera.rotation
                    * state
                        .camera
                        .camera_pixel_to_viewport_distance(x as f32, y as f32);

                let chr = trace_ray(state.camera.position, d, 1.0, f32::INFINITY, &state.spheres);

                state.camera.buffer[(y + (height / 2)) as usize][(x + (width / 2)) as usize] = chr;
            }
        }

        state.count = 0.0;

        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let color = if state.is_alive(x, y) {
                    Color::RED
                } else {
                    Color::WHITE
                };

                state.set_color(color, x, y);
            }
        }

        state.swap_data();
    }
}

fn draw(gfx: &mut Graphics, state: &mut State) {
    // Update the texture with the new data
    let mut draw = gfx.create_draw();
    draw.clear(Color::BLACK);

    if state.dirty {
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                state.buffer[y * WIDTH + x] = if state.is_alive(x, y) { 'X' } else { '.' };
            }
            // state.buffer[(y + 1) * WIDTH - 1] = '\n';
        }
        state.dirty = false;
    }

    let mut display = String::new();
    for line in state.camera.buffer.iter().rev() {
        display.extend(line.iter());
        display = display + "\n";
    }

    draw.text(&state.font, &display)
        .position(0.0, 0.0)
        .size(16.0);

    gfx.render(&draw);
}

fn index(x: isize, y: isize) -> Option<usize> {
    if x < 0 || y < 0 {
        return None;
    }

    let x = x as usize;
    let y = y as usize;
    let index = ((y * WIDTH) + x) * 4;
    if index >= BYTES_LENGTH {
        None
    } else {
        Some(index)
    }
}

#[inline]
fn is_red_color(bytes: &[u8; 4]) -> bool {
    bytes == &[255, 0, 0, 255]
}

#[rustfmt::skip]
fn get_neighbors(ix: isize, iy: isize) -> [(isize, isize); 8] {
    [
        (ix - 1, iy - 1), (ix, iy - 1), (ix + 1, iy - 1),
        (ix - 1, iy),                   (ix + 1, iy),
        (ix - 1, iy + 1), (ix, iy + 1), (ix + 1, iy + 1),
    ]
}
