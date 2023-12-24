use notan::math::Mat3;
use notan::math::Vec3;
use notan::prelude::*;
use notan::text::*;
use rayon::prelude::*;

const WIDTH: usize = 1920;
const HEIGHT: usize = 1080;

const ROWS: usize = HEIGHT / 16;
const COLS: usize = WIDTH / 8;

// The constant 'D' represents the distance between the camera and the projection plane.
const D: f32 = 1.0;

struct Triangle {
    vertex1: Vec3,
    vertex2: Vec3,
    vertex3: Vec3,
}

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
    buffer: Vec<char>,
}

impl Camera {
    fn camera_pixel_to_viewport_distance(&self, x: f32, y: f32) -> Vec3 {
        Vec3 {
            x: x * self.viewport.width / COLS as f32,
            y: y * self.viewport.height / ROWS as f32,
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
    let win_config = WindowConfig::new()
        .set_size(WIDTH as u32, HEIGHT as u32)
        .set_title("Cast")
        .set_vsync(true)
        .set_resizable(true)
        .set_min_size(600, 400);

    notan::init_with(setup)
        .initialize(init)
        .add_config(win_config)
        .add_config(TextConfig)
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
        buffer: Vec::with_capacity(COLS * ROWS),
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

fn ray_intersects_triangle(
    ray_origin: Vec3,
    ray_direction: Vec3,
    triangle: &Triangle,
) -> Option<(Vec3, Vec3)> {
    const EPSILON: f32 = 1e-6;

    let triangle_normal = (triangle.vertex2 - triangle.vertex1)
        .cross(triangle.vertex3 - triangle.vertex1)
        .normalize();

    let triangle_d = -triangle_normal.dot(triangle.vertex1);

    let denominator = ray_direction.dot(triangle_normal);

    if denominator.abs() < EPSILON {
        return None; // Ray is parallel to the triangle plane
    }

    let t = -(triangle_normal.dot(ray_origin) + triangle_d) / denominator;

    if t < EPSILON {
        return None; // Intersection point is behind the ray origin
    }

    let intersection_point = ray_origin + ray_direction * t;

    // Check if the intersection point is inside the triangle using barycentric coordinates
    let e1 = triangle.vertex2 - triangle.vertex1;
    let e2 = triangle.vertex3 - triangle.vertex1;
    let q = intersection_point - triangle.vertex1;

    let u = q.dot(e1) / e1.length_squared();
    let v = q.dot(e2) / e2.length_squared();

    if u >= 0.0 && v >= 0.0 && u + v <= 1.0 {
        Some((intersection_point, triangle_normal))
    } else {
        None
    }
}

fn ray_intersects_cuboid_no_rotation(
    origin: Vec3,
    direction: Vec3,
    position: Vec3,
    half_extents: Vec3,
) -> Option<(Vec3, Vec3)> {
    let inv_direction = Vec3::new(1.0 / direction.x, 1.0 / direction.y, 1.0 / direction.z);

    let t1 = (position - origin) * inv_direction;
    let t2 = (position + half_extents - origin) * inv_direction;

    let tmin = t1.min(t2);
    let tmax = t1.max(t2);

    let t_enter = tmin.max_element();
    let t_exit = tmax.min_element();

    if t_exit < 0.0 || t_enter > t_exit {
        return None; // No intersection or behind the ray origin
    }

    let intersection_point = origin + direction * t_enter;
    let normal = compute_cuboid_normal(intersection_point, position, half_extents);

    Some((intersection_point, normal))
}

fn compute_cuboid_normal(point: Vec3, position: Vec3, half_extents: Vec3) -> Vec3 {
    let local_point = point - position;
    let mut normal = Vec3::default();

    for i in 0..3 {
        if local_point[i].abs() + 1e-6 > half_extents[i] {
            normal[i] = local_point[i].signum();
        }
    }

    normal
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

fn compute_lighting(p: Vec3, n: Vec3, player_pos: Vec3) -> char {
    let mut i = 0.2;

    // let light_pos = Vec3 {
    //     x: 2.0,
    //     y: 1.0,
    //     z: -3.0,
    // };
    let light_pos = player_pos;

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

    let triangle = Triangle {
        vertex1: Vec3::new(0.0, -1.0, 1.0),
        vertex2: Vec3::new(3.0, -1.0, -1.0),
        vertex3: Vec3::new(1.0, 2.0, 1.0),
    };

    if let Some((intersection_point, normal)) =
        ray_intersects_triangle(origin, direction, &triangle)
    {
        if intersection_point.length() < closest_t {
            return compute_lighting(intersection_point, normal.normalize(), origin);
        }
    }

    // Cuboid transformation (rotation, translation, etc.)
    let cuboid_position = Vec3::new(-1.0, 0.0, 3.0);
    let cuboid_half_extents = Vec3::new(1.0, 1.0, 1.0); // Half extents along each axis

    let pp =
        ray_intersects_cuboid_no_rotation(origin, direction, cuboid_position, cuboid_half_extents);
    if let Some((pt, nt)) = pp {
        if pt.length() < closest_t {
            return compute_lighting(pt, nt / nt.length(), origin);
        }
    }

    if let Some(s) = closest_sphere {
        let p = origin + closest_t * direction;
        let n = p - s.center;

        return compute_lighting(p, n / n.length(), origin);
    }

    ' '
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

    let rows = ROWS as i32;
    let cols = COLS as i32;
    state.camera.buffer = (0..rows * cols)
        .into_par_iter()
        .map(|i| {
            let x = (i % cols) - (cols / 2);
            let y = (i / cols) - (rows / 2);

            let position = state.camera.position;
            let rotation = state.camera.rotation;
            let direction: Vec3 = rotation
                * state
                    .camera
                    .camera_pixel_to_viewport_distance(x as f32, y as f32);

            trace_ray(position, direction, 1.0, f32::INFINITY, &state.spheres)
        })
        .collect();
}

fn draw(app: &mut App, gfx: &mut Graphics, state: &mut State) {
    let mut text = gfx.create_text();
    text.clear_options(ClearOptions::color(Color::BLACK));

    let display: String = state
        .camera
        .buffer
        .par_chunks(COLS)
        .map(|chunk: &[char]| chunk.iter().collect::<String>() + "\n")
        .rev()
        .collect();

    text.add(&display).font(&state.font);

    // TODO: This seems to be a bottlekneck... presumably the notan text rendering
    // isn't intended to be used like this.
    // IDEA: Could try to pre-render all the light values to textures and stitch
    // them together somehow?
    gfx.render(&text);

    println!("fps: {}", app.timer.fps().round());
}
