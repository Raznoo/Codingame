use std::cell::RefCell;
use std::cmp::PartialEq;
use std::io;
use std::rc::Rc;

macro_rules! parse_input {
    ($x:expr, $t:ident) => {
        $x.trim().parse::<$t>().unwrap()
    };
}
#[derive(Debug)]
struct Checkpoint {
    x: i32,
    y: i32,
    angle: i32,
    dist: i32,
    is_best: bool,
}

impl Checkpoint {
    fn new(x: i32, y: i32) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Checkpoint {
            x,
            y,
            angle: -1,
            dist: -1,
            is_best: false,
        }))
    }

    fn fake_new() -> Rc<RefCell<Self>> {
        Checkpoint::new(-1, -1)
    }
    fn new_player_cp(player: &Player) -> Self {
        Checkpoint {
            x: player.curr_x,
            y: player.curr_y,
            angle: -1,
            dist: -1,
            is_best: false,
        }
    }
}
impl Clone for Checkpoint {
    fn clone(&self) -> Checkpoint {
        Checkpoint {
            x: self.x,
            y: self.y,
            angle: -1,
            dist: -1,
            is_best: self.is_best,
        }
    }
}
impl PartialEq for Checkpoint {
    fn eq(&self, other: &Checkpoint) -> bool {
        self.x == other.x && self.y == other.y
    }
}
impl Checkpoint {
    fn distance_to(&self, other: &Self) -> f32 {
        let abs_x = (self.x - other.x).abs() as f32;
        let abs_y = (self.y - other.y).abs() as f32;
        let result = (abs_x.powi(2) + abs_y.powi(2)).sqrt();
        return result;
    }

    fn update_from_player(&mut self, player: &Player) {
        let delta_x = self.x - player.curr_x;
        let delta_y = self.y - player.curr_y;

        // Calculate the absolute angle from the player to the checkpoint.
        let mut absolute_angle_to_checkpoint =
            (delta_y as f32).atan2(delta_x as f32) * 180.0 / std::f32::consts::PI;
        if absolute_angle_to_checkpoint < 0.0 {
            absolute_angle_to_checkpoint += 360.0;
        }

        // Compute relative angle
        let mut relative_angle = absolute_angle_to_checkpoint - player.angle as f32;
        if relative_angle > 180.0 {
            relative_angle -= 360.0;
        } else if relative_angle < -180.0 {
            relative_angle += 360.0;
        }

        self.angle = relative_angle as i32;
    }
}
struct Player {
    curr_x: i32,
    curr_y: i32,
    angle: i32,
    delta_x: i32,
    delta_y: i32,
}

impl Player {
    fn new() -> Player {
        Player {
            curr_x: -1,
            curr_y: -1,
            angle: -1,
            delta_x: -1,
            delta_y: -1,
            // has_boost: false,
        }
    }

    fn tick(&mut self, new_pos: (i32, i32, i32)) {
        self.angle = new_pos.2;
        self.delta_x = new_pos.0 - self.curr_x;
        self.delta_y = new_pos.1 - self.curr_y;
        self.curr_x = new_pos.0;
        self.curr_y = new_pos.1;
    }
}

struct MapState {
    first_lap: bool,
    checkpoints: Vec<Rc<RefCell<Checkpoint>>>,
    me: Player,
    has_boost: bool,

    curr_cp: Rc<RefCell<Checkpoint>>,
    curr_cp_ind: usize,
    next_cp_ind: usize,
}

impl MapState {
    fn new() -> MapState {
        MapState {
            first_lap: true,
            has_boost: true,
            checkpoints: Vec::new(),
            me: Player::new(),

            curr_cp: Checkpoint::fake_new(),
            curr_cp_ind: 0,
            next_cp_ind: 1,
        }
    }
    fn compute_three_point_angle(
        &self,
        prev: &Checkpoint,
        curr: &Checkpoint,
        next: &Checkpoint,
    ) -> f32 {
        fn distance(p1: &Checkpoint, p2: &Checkpoint) -> f32 {
            let dx = (p1.x - p2.x) as f32;
            let dy = (p1.y - p2.y) as f32;
            (dx * dx + dy * dy).sqrt()
        }
        // Calculate the distances (or sides of the triangle)
        let a = distance(curr, next);
        let b = distance(curr, prev);
        let c = distance(prev, next);

        // Compute the angle using the law of cosines
        let cos_theta = (a * a + b * b - c * c) / (2.0 * a * b);

        // Return the angle in degrees

        return cos_theta.acos() * 180.0 / std::f32::consts::PI;
    }

    fn determine_best_cp(&self) {
        //index is needed for overflow magic
        let mut max_cp = self.checkpoints.get(0).unwrap();
        let mut max_dist = f32::MIN;
        for (i, cp) in self.checkpoints.iter().enumerate() {
            let next_i = if i == self.checkpoints.len() - 1 {
                0
            } else {
                i + 1
            };
            let next_cp = self.checkpoints.get(next_i).unwrap();
            let dist = cp.borrow().distance_to(&next_cp.borrow());
            if dist > max_dist {
                max_dist = dist;
                max_cp = next_cp;
            }
        }
        max_cp.borrow_mut().is_best = true;
    }

    fn determine_target(&self) -> String {
        let result_point = (
            self.curr_cp.borrow().x - (self.me.delta_x * 3),
            self.curr_cp.borrow().y - (self.me.delta_y * 3),
        );
        let result = format!("{} {} ", result_point.0, result_point.1);
        return result;
    }

    fn determine_accel(&mut self) -> String {
        let current_angle = self.curr_cp.borrow().angle as f32;
        // Calculate distance slowdown factor
        let distance_to_checkpoint_sqr = self.curr_cp.borrow().dist as f32;
        let distance_slowdown = f32::min(distance_to_checkpoint_sqr / 36000.0, 1.0);

        // Check if we can use boost
        if self.has_boost && current_angle.abs() < 10.0 && self.curr_cp.borrow().is_best {
            self.has_boost = false;
            return "BOOST".to_string();
        }
        let computed_angle = self.compute_three_point_angle(
            &Checkpoint::new_player_cp(&self.me),
            &self.curr_cp.borrow(),
            &self.checkpoints[self.next_cp_ind].borrow(),
        );
        // Calculate angle slowdown factor
        let angle_slowdown_factor = 1.0 - f32::min(computed_angle / 90.0, 1.0);
        // Determine thrust based on the computed angle and slowdown factors
        let thrust_base = (100.0 as f32 * (1.0 - computed_angle / 180.0)) as i32;
        let thrust = 100 - (thrust_base as f32 * distance_slowdown * angle_slowdown_factor) as i32;
        eprintln!("distance_to_checkpoint_sqr: {}", distance_to_checkpoint_sqr);
        eprintln!("distance_slowdown: {}", distance_slowdown);
        eprintln!("computed_angle: {}", computed_angle);
        eprintln!("thrust_base: {}", thrust_base);
        eprintln!("thrust: {}", thrust);
        eprintln!("{} : {}", distance_slowdown, angle_slowdown_factor);
        eprintln!("Taking angle: {:?}", computed_angle);
        thrust.to_string()

        // let slowdown_threshold = 2000;
    }

    fn map_tick(&mut self, player_input: (i32, i32, i32, i32)) -> String {
        // Update player's position (logically not physically).
        self.me
            .tick((player_input.0, player_input.1, player_input.2));

        if self.curr_cp_ind != player_input.3 as usize {
            
            self.curr_cp_ind = self.next_cp_ind;
            self.next_cp_ind = (self.next_cp_ind + 1) % self.checkpoints.len();
            self.curr_cp = Rc::clone(&self.checkpoints[self.curr_cp_ind]);
        }

        for cp in self.checkpoints.iter() {
            cp.borrow_mut().update_from_player(&self.me);
        }

        // self.curr_cp.borrow_mut().update_from_player(&self.me);
        // self.curr_cp.borrow_mut().is_best = self.checkpoints[self.curr_cp_ind].borrow().is_best;

        let mut result = self.determine_target();
        result.push_str(&self.determine_accel());

        eprintln!("current: {:?}", self.curr_cp);
        result
    }
}

fn main() {
    let mut map1 = MapState::new();
    let mut map2 = MapState::new();

    let mut input_line = String::new();
    io::stdin().read_line(&mut input_line).unwrap();
    let laps = parse_input!(input_line, i32);
    let mut input_line = String::new();
    io::stdin().read_line(&mut input_line).unwrap();
    let checkpoint_count = parse_input!(input_line, i32);
    for i in 0..checkpoint_count as usize {
        let mut input_line = String::new();
        io::stdin().read_line(&mut input_line).unwrap();
        let inputs = input_line.split(" ").collect::<Vec<_>>();
        let checkpoint_x = parse_input!(inputs[0], i32);
        let checkpoint_y = parse_input!(inputs[1], i32);
        let new_cp = Checkpoint::new(checkpoint_x, checkpoint_y);
        map1.checkpoints.push(Rc::clone(&new_cp));
        map2.checkpoints.push(Rc::clone(&new_cp));
    }

    // game loop
    loop {
        for i in 0..2 as usize {
            let mut input_line = String::new();
            io::stdin().read_line(&mut input_line).unwrap();
            let inputs = input_line.split(" ").collect::<Vec<_>>();
            let x = parse_input!(inputs[0], i32); // x position of your pod
            let y = parse_input!(inputs[1], i32); // y position of your pod
            let vx = parse_input!(inputs[2], i32); // x speed of your pod
            let vy = parse_input!(inputs[3], i32); // y speed of your pod
            let angle = parse_input!(inputs[4], i32); // angle of your pod
            eprintln!("{}", angle);
            let next_check_point_id = parse_input!(inputs[5], i32); // next check point id of your pod
            let player_input = (x, y, angle, next_check_point_id);
            let results = if i == 0 {
                map1.map_tick(player_input)
            } else {
                map2.map_tick(player_input)
            };
            println!("{}", results);
        }
        for i in 0..2 as usize {
            //here be monsters
            let mut input_line = String::new();
            io::stdin().read_line(&mut input_line).unwrap();
            let inputs = input_line.split(" ").collect::<Vec<_>>();
            let x_2 = parse_input!(inputs[0], i32); // x position of the opponent's pod
            let y_2 = parse_input!(inputs[1], i32); // y position of the opponent's pod
            let vx_2 = parse_input!(inputs[2], i32); // x speed of the opponent's pod
            let vy_2 = parse_input!(inputs[3], i32); // y speed of the opponent's pod
            let angle_2 = parse_input!(inputs[4], i32); // angle of the opponent's pod
            let next_check_point_id_2 = parse_input!(inputs[5], i32); // next check point id of the opponent's pod
        }
    }
}
