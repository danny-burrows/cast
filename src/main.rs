use notan::draw::*;
use notan::math::Mat3;
use notan::math::Vec3;
use notan::prelude::*;

const WIDTH: usize = 1920;
const HEIGHT: usize = 1080;

const ROWS: usize = HEIGHT / 16;
const COLS: usize = WIDTH / 8;

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
    buffer: [[char; COLS]; ROWS],
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
}

#[notan_main]
fn main() -> Result<(), String> {
    let win_config = WindowConfig::new().set_size(WIDTH as u32, HEIGHT as u32);

    notan::init_with(setup)
        .initialize(init)
        .add_config(win_config)
        .add_config(DrawConfig)
        .update(update)
        .draw(draw)
        .build()
}

fn setup(gfx: &mut Graphics) -> State {
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
        buffer: [[' '; COLS]; ROWS],
    };

    State {
        font,
        camera,
        spheres: Vec::new(),
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
    scale[index]
}

fn trace_ray(origin: Vec3, direction: Vec3, t_min: f32, t_max: f32, spheres: &[Sphere]) -> char {
    let mut closest_t: f32 = f32::INFINITY;
    let mut closest_sphere: Option<&Sphere> = None;

    for sphere in spheres {
        let (t1, t2) = ray_intersects_sphere(origin, direction, sphere);

        if t_min < t1 && t1 < t_max && t1 < closest_t {
            closest_t = t1;
            closest_sphere = Some(sphere);
        }
        if t_min < t2 && t2 < t_max && t2 < closest_t {
            closest_t = t2;
            closest_sphere = Some(sphere);
        }
    }

    if let Some(s) = closest_sphere {
        let p = origin + closest_t * direction;
        let n = p - s.center;

        compute_lighting(p, n / n.length())
    } else {
        ' '
    }
}

fn update(app: &mut App, state: &mut State) {
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

    let (width, height): (i32, i32) = (COLS as i32, ROWS as i32);

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
}

fn draw(gfx: &mut Graphics, state: &mut State) {
    // Update the texture with the new data
    let mut draw = gfx.create_draw();
    draw.clear(Color::BLACK);

    let mut display = String::new();
    for line in state.camera.buffer.iter().rev() {
        display.extend(line.iter());
        display += "\n";
    }

    draw.text(&state.font, &display)
        .position(0.0, 0.0)
        .size((HEIGHT / ROWS) as f32);

    gfx.render(&draw);
}
