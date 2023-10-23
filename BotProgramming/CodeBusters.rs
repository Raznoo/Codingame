extern crate rand;
use rand::Rng;
use std::cell::RefCell;
use std::cmp::PartialEq;
use std::fmt::format;
use std::io;
use std::rc::Rc;

const MAP_RIGHT_WALL: i32 = 16000;
const MAP_BOTTOM_WALL: i32 = 9000;

const OUTER_BUSTING_RADIUS: i32 = 1750;
const INNER_BUSTING_RADIUS: i32 = 900;

const BASE_RELEASE_RADIUS: i32 = 1600;
const LINE_OF_SIGHT_RADIUS: i32 = 2200;

const CAPTURE_RADIUS: i32 = 6500;
const ORIGIN_THROWING_OFFSET: i32 = 500;
const THROW_MARGIN: f64 = 0.9;
const CAMPING_MARGIN: f64 = 0.1;

const START_CAMPING_TURN: i32 = 100;
const CAMPING_RADIUS: i32 = 2500;
const TURNS_UNTIL_GLOBAL_PICKS: i32 = 50;
const STUN_COOLDOWN: i32 = 20;

macro_rules! parse_input {
    ($x:expr, $t:ident) => {
        $x.trim().parse::<$t>().unwrap()
    };
}
#[derive(Debug)]
struct Coord {
    x: i32,
    y: i32,
}

impl Coord {
    fn new(x: i32, y: i32) -> Coord {
        Coord { x: x, y: y }
    }
    fn fake_new() -> Coord {
        Coord::new(-1, -1)
    }
}

impl PartialEq for Coord {
    //is equivalent
    fn eq(&self, other: &Coord) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl Clone for Coord {
    fn clone(&self) -> Coord {
        Coord {
            x: self.x,
            y: self.y,
        }
    }
}

impl Coord {
    fn distance_to(&self, other: &Coord) -> i32 {
        let result =
            (((self.x - other.x).abs().pow(2) + (self.y - other.y).abs().pow(2)) as f32).sqrt();
        return result.round() as i32;
    }
    fn is_in_radius(&self, other: &Coord, radius: i32) -> bool {
        return self.distance_to(other) <= radius;
    }
    fn to_string(&self) -> String {
        return format!("{} {}", self.x, self.y);
    }
    fn set(&mut self, other: &Coord) -> () {
        self.x = other.x;
        self.y = other.y;
    }
    fn interpolate(&self, other: &Self, factor: f64) -> Self {
        let x = self.x as f64 + factor * (other.x as f64 - self.x as f64);
        let y = self.y as f64 + factor * (other.y as f64 - self.y as f64);
        Coord::new(x.round() as i32, y.round() as i32)
    }
    fn throw_coords(
        &self,
        count: usize,
        down_right: bool,
        radius: i32,
        is_camping: bool,
    ) -> Vec<Coord> {
        let mut results = Vec::new();

        // Determine the center of the quadrant
        let quadrant_center = if down_right {
            Coord::new(self.x - radius, self.y - radius)
        } else {
            Coord::new(self.x + radius, self.y + radius)
        };

        for i in 0..count {
            let angle = (i as f64) / ((count - 1) as f64) * std::f64::consts::FRAC_PI_2;

            // Points on the quadrant circumference
            let dx = radius as f64 * angle.cos();
            let dy = radius as f64 * angle.sin();
            let point_on_circumference = if down_right {
                Coord::new((self.x - dx.round() as i32), (self.y - dy.round() as i32))
            } else {
                Coord::new((self.x + dx.round() as i32), (self.y + dy.round() as i32))
            };

            // Interpolate between the point on the circumference and the quadrant center
            let throw_margin = if is_camping {
                CAMPING_MARGIN
            } else {
                THROW_MARGIN
            };
            let adjusted_point = point_on_circumference.interpolate(&quadrant_center, throw_margin);

            results.push(adjusted_point);
        }

        results
    }
}

#[derive(Debug)]
struct Ghost {
    entity_id: i32,
    coords: Coord,
    people_trapping: i32,
    stamina: i32,
}

impl Ghost {
    // constructors / updaters
    fn new(id: i32, x: i32, y: i32, people_trapping: i32, stamina: i32) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Ghost {
            entity_id: id,
            coords: Coord::new(x, y),
            people_trapping,
            stamina,
        }))
    }
    fn transfer_ghost_data(&mut self, other: &Self) -> () {
        self.coords = Coord::new(other.coords.x, other.coords.y);
        self.people_trapping = other.people_trapping;
        self.stamina = other.stamina;
        self.people_trapping = other.people_trapping;
    }
}

impl Clone for Ghost {
    fn clone(&self) -> Ghost {
        Ghost {
            entity_id: self.entity_id,
            coords: Coord {
                x: self.coords.x,
                y: self.coords.y,
            },
            people_trapping: self.people_trapping,
            stamina: self.stamina,
        }
    }
}

impl Ghost {
    //functionality
    fn is_bustable(&self, buster: &Buster) -> bool {
        return self
            .coords
            .is_in_radius(&buster.coords, OUTER_BUSTING_RADIUS)
            && !self
                .coords
                .is_in_radius(&buster.coords, INNER_BUSTING_RADIUS);
    }
    fn is_too_close(&self, buster: &Buster) -> bool {
        return self
            .coords
            .is_in_radius(&buster.coords, INNER_BUSTING_RADIUS);
    }
}

impl PartialEq for Ghost {
    //is equivalent
    fn eq(&self, other: &Ghost) -> bool {
        self.entity_id == other.entity_id
    }
}

#[derive(Copy, Clone, Debug)]
enum BusterState {
    //buster states
    Idle,
    // should only happen first turn
    Searching,
    // looking for ghost
    MovingToGhost,
    // found a ghost, moving towards it
    Busting,
    // in bustable range for ghost, wanting to bust
    MovingToHome,
    // have a ghost, taking it home
    Releasing,
    // made it home with ghost, letting it out
    MovingToAsshole,
    // wanting to stun someone and found a target
    Stunning,
    // in stunning range for person, wanting to stun
    MovingAwayFromGhost,
    // in ghost's inner circle, need to move away
    Stunned,

    //camping states
    Camping,
    MovingToCamp,

    //opponent states
    NotTargetted,
    // this isn't used rn. maybe use for death mark
    DeathMarked, // this isn't used rn. maybe use for death mark
}

#[derive(Debug)]
struct Buster {
    entity_id: i32,
    coords: Coord,
    state: BusterState,
    busting_target_ref: Option<Rc<RefCell<Ghost>>>,
    stunning_target_ref: Option<Rc<RefCell<Buster>>>,
    movement_target: Coord,
    stun_timer: i32,
    has_ghost: bool,
    is_stunned: bool,
}

impl Buster {
    fn new(entity_id: i32, x: i32, y: i32, has_ghost: bool, is_stunned: bool) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Buster {
            entity_id,
            coords: Coord::new(x, y),
            state: BusterState::Idle,
            busting_target_ref: None,
            stunning_target_ref: None,
            movement_target: Coord::fake_new(),
            stun_timer: 0,
            has_ghost,
            is_stunned,
        }))
    }
    fn transfer_buster_data(&mut self, other: &Self) -> () {
        self.coords = Coord::new(other.coords.x, other.coords.y);
        self.has_ghost = other.has_ghost;
        self.is_stunned = other.is_stunned;
    }
    fn new_opponent(
        entity_id: i32,
        x: i32,
        y: i32,
        has_ghost: bool,
        is_stunned: bool,
    ) -> Rc<RefCell<Self>> {
        let new_opp = Buster::new(entity_id, x, y, has_ghost, is_stunned);
        new_opp.borrow_mut().state = BusterState::NotTargetted;
        return new_opp;
    }
}

impl Buster {
    fn tick(&mut self) -> () {
        if self.stun_timer > 0 {
            self.stun_timer -= 1;
        }
    }
    fn can_stun(&self) -> bool {
        return self.stun_timer == 0;
    }
    fn is_stunnable(&self, buster: &Buster) -> bool {
        return self
            .coords
            .is_in_radius(&buster.coords, OUTER_BUSTING_RADIUS);
    }
}

impl PartialEq for Buster {
    //is equivalent
    fn eq(&self, other: &Buster) -> bool {
        self.entity_id == other.entity_id
    }
}

impl Clone for Buster {
    fn clone(&self) -> Buster {
        Buster {
            entity_id: self.entity_id,
            coords: Coord::new(self.coords.x, self.coords.y),
            state: self.state,
            busting_target_ref: if let Some(ghost) = &self.busting_target_ref {
                Some(Rc::clone(&ghost))
            } else {
                None
            },
            stunning_target_ref: if let Some(asshole) = &self.stunning_target_ref {
                Some(Rc::clone(&asshole))
            } else {
                None
            },
            stun_timer: self.stun_timer,
            movement_target: Coord::new(self.movement_target.x, self.movement_target.y),
            has_ghost: self.has_ghost,
            is_stunned: self.is_stunned,
        }
    }
}

struct StateMachine {
    upper_left_home: bool,
    all_ghosts: Vec<Rc<RefCell<Ghost>>>,
    good_busters: Vec<Rc<RefCell<Buster>>>,
    bad_busters: Vec<Rc<RefCell<Buster>>>,
    global_ping_stack: Vec<Rc<RefCell<Ghost>>>,
    turn_count: i32,
}

impl StateMachine {
    //other functions
    fn throw_and_assign_coords(&self, is_camping: bool) -> () {
        let num_busters = self.good_busters.len();
        let origin = if self.upper_left_home != is_camping {
            Coord::new(ORIGIN_THROWING_OFFSET, ORIGIN_THROWING_OFFSET)
        } else {
            Coord::new(
                MAP_RIGHT_WALL - ORIGIN_THROWING_OFFSET,
                MAP_BOTTOM_WALL - ORIGIN_THROWING_OFFSET,
            )
        };
        let mut thrown_coords = origin.throw_coords(
            num_busters,
            if is_camping {
                self.upper_left_home
            } else {
                !self.upper_left_home
            },
            if is_camping {
                CAMPING_RADIUS
            } else {
                CAPTURE_RADIUS
            },
            is_camping,
        );

        for buster in &mut self.good_busters.iter() {
            let mut best_index = None;
            let mut min_dist = i32::MAX;
            for (index, coord) in thrown_coords.iter().enumerate() {
                let dist = buster.borrow().coords.distance_to(coord);
                if dist < min_dist {
                    best_index = Some(index);
                    min_dist = dist;
                }
            }

            if let Some(index) = best_index {
                let best_coord = thrown_coords.remove(index);
                buster
                    .as_ref()
                    .borrow_mut()
                    .movement_target
                    .set(&best_coord);
            }
        }
    }
    fn set_new_buster_movement_coords(&self, buster: &mut Buster) -> () {
        // lol its just a rng
        let mut rng = rand::thread_rng();
        let new_coords = Coord::new(
            rng.gen_range(0..MAP_RIGHT_WALL),
            rng.gen_range(0..MAP_BOTTOM_WALL),
        );
        buster.movement_target = new_coords;
    }
    fn set_asshole_target(&self, asshole: &Rc<RefCell<Buster>>, buster: &mut Buster) -> () {
        asshole.borrow_mut().state = BusterState::DeathMarked;
        buster.stunning_target_ref = Some(Rc::clone(asshole));
        if let Some(_) = &buster.busting_target_ref {
            buster.busting_target_ref = None;
        }
    }
    fn find_ghost_in_radius_of_buster(&self, buster: &Buster) -> Option<&Rc<RefCell<Ghost>>> {
        return self.all_ghosts.iter().find(|ghost| {
            ghost
                .borrow()
                .coords
                .is_in_radius(&buster.coords, LINE_OF_SIGHT_RADIUS)
        });
    }

    fn do_global_ping(&mut self) {
        let mut temp = Vec::new();
        for buster_rc in &self.good_busters {
            let buster = buster_rc.borrow();
            if let Some(ref ghost_rc) = buster.busting_target_ref {
                temp.push(Rc::clone(ghost_rc));
            }
        }
        self.global_ping_stack = temp;
    }

    fn find_best_ghost_in_los(&self, buster: &Buster) -> Option<&Rc<RefCell<Ghost>>> {
        self.all_ghosts
            .iter()
            .filter(|&ghost_rc| {
                let ghost = ghost_rc.borrow();
                buster.coords.distance_to(&ghost.coords) <= LINE_OF_SIGHT_RADIUS
            })
            .min_by_key(|&ghost_rc| {
                let ghost = ghost_rc.borrow();
                std::cmp::max(ghost.stamina, 3)
            })
    }
    fn find_best_asshole_in_los(&self, buster: &Buster) -> Option<Rc<RefCell<Buster>>> {
        let within_radius_assholes: Vec<_> = self
            .bad_busters
            .iter()
            .filter(|&buster_rc| {
                let distance = buster.coords.distance_to(&buster_rc.borrow().coords);
                distance <= LINE_OF_SIGHT_RADIUS
                    && !matches!(buster_rc.borrow().state, BusterState::DeathMarked)
                    && buster.can_stun()
                    && !buster_rc.borrow().is_stunned
            })
            .collect();

        let asshole_with_ghost = within_radius_assholes
            .iter()
            .find(|&buster_rc| buster_rc.borrow().has_ghost);

        match asshole_with_ghost {
            Some(asshole) => Some(Rc::clone(asshole)),
            None => within_radius_assholes
                .get(0)
                .map(|&buster_rc| Rc::clone(buster_rc)),
        }
    }
    fn steal_target_ghost_reference(&self, buster: &Buster) -> Option<Rc<RefCell<Ghost>>> {
        let mut closest_ghost: Option<Rc<RefCell<Ghost>>> = None;
        let mut min_dist = i32::MAX;
        if self.turn_count < TURNS_UNTIL_GLOBAL_PICKS {
            return None;
        }
        // Iterating through the global ping stack to find the closest ghost
        for target_ghost_rc in &self.global_ping_stack {
            let distance = buster.coords.distance_to(&target_ghost_rc.borrow().coords);
            if distance < min_dist {
                closest_ghost = Some(Rc::clone(target_ghost_rc));
                min_dist = distance;
            }
        }
        closest_ghost
    }
    fn ghost_difficulty_test(&self, ghost: &Ghost) -> bool {
        eprintln!("{:?}", ghost);
        return match ghost.stamina {
            x if x > 15 => {
                eprintln!("made it here");
                eprintln!("{}", self.turn_count >= TURNS_UNTIL_GLOBAL_PICKS);
                self.turn_count >= TURNS_UNTIL_GLOBAL_PICKS
            }
            _ => true,
        };
    }
}

impl StateMachine {
    //constructors and updators
    fn new(team_id: i32) -> StateMachine {
        StateMachine {
            upper_left_home: team_id == 0,
            all_ghosts: Vec::new(),
            good_busters: Vec::new(),
            bad_busters: Vec::new(),
            global_ping_stack: Vec::new(),
            turn_count: 0,
        }
    }

    fn update_ghosts(&mut self, new_ghosts: Vec<Rc<RefCell<Ghost>>>) {
        let mut to_remove = Vec::new();

        // Update data for old ghosts and mark absent old ghosts for removal
        for (index, old_ghost_rc) in self.all_ghosts.iter().enumerate() {
            if new_ghosts
                .iter()
                .find(|&new_ghost_rc| old_ghost_rc.borrow().eq(&new_ghost_rc.borrow()))
                .is_none()
            {
                to_remove.push(index);
            }
        }

        // Remove old ghosts not present in new_ghosts
        for index in to_remove.into_iter().rev() {
            let ghost = &self.all_ghosts[index];

            // Check if this ghost is being targeted by any good_buster
            for buster_rc in &self.good_busters {
                let mut buster = buster_rc.borrow_mut();
                if let Some(target) = &buster.busting_target_ref {
                    if Rc::ptr_eq(target, ghost) {
                        buster.busting_target_ref = None;
                    }
                }
            }
            self.all_ghosts.remove(index);
        }

        for new_ghost_rc in new_ghosts.iter() {
            if self
                .all_ghosts
                .iter()
                .find(|&old_ghost_rc| old_ghost_rc.borrow().eq(&new_ghost_rc.borrow()))
                .is_none()
            {
                self.all_ghosts.push(new_ghost_rc.clone());
            } else {
                let old_ghost_rc = self
                    .all_ghosts
                    .iter()
                    .find(|&old_ghost_rc| old_ghost_rc.borrow().eq(&new_ghost_rc.borrow()))
                    .unwrap();
                old_ghost_rc
                    .borrow_mut()
                    .transfer_ghost_data(&new_ghost_rc.borrow());
            }
        }
    }

    fn update_good(&mut self, new_busters: Vec<Rc<RefCell<Buster>>>) -> () {
        for old_buster_rc in self.good_busters.iter() {
            if let Some(new_buster_rc) = new_busters
                .iter()
                .find(|&new_buster_rc| old_buster_rc.borrow().eq(&new_buster_rc.borrow()))
            {
                old_buster_rc
                    .borrow_mut()
                    .transfer_buster_data(&new_buster_rc.borrow());
            }
        }
    }

    fn update_evil(&mut self, new_busters: Vec<Rc<RefCell<Buster>>>) -> () {
        // Indices of bad busters that need to be removed
        let mut to_remove = Vec::new();

        for (index, old_buster_rc) in self.bad_busters.iter().enumerate() {
            if let Some(new_buster_rc) = new_busters
                .iter()
                .find(|&new_buster_rc| old_buster_rc.borrow().eq(&new_buster_rc.borrow()))
            {
                old_buster_rc
                    .borrow_mut()
                    .transfer_buster_data(&new_buster_rc.borrow());
            } else {
                to_remove.push(index);
            }
        }

        for index in to_remove.into_iter().rev() {
            let evil_buster = &self.bad_busters[index];

            for good_buster_rc in &self.good_busters {
                let mut good_buster = good_buster_rc.borrow_mut();
                if let Some(stunning_target_ref) = &good_buster.stunning_target_ref {
                    if Rc::ptr_eq(stunning_target_ref, evil_buster) {
                        good_buster.stunning_target_ref = None;
                    }
                }
            }

            self.bad_busters.remove(index);
        }

        // Append any new busters not present in bad_busters
        for new_buster_rc in new_busters.iter() {
            if self
                .bad_busters
                .iter()
                .find(|&old_buster_rc| old_buster_rc.borrow().eq(&new_buster_rc.borrow()))
                .is_none()
            {
                self.bad_busters.push(new_buster_rc.clone());
            }
        }
    }
}

impl StateMachine {
    //what to do for each state
    fn do_state(&self, mut buster: std::cell::RefMut<'_, Buster>) -> String {
        let mut result = match &buster.state {
            BusterState::Idle => unreachable!(),
            BusterState::Searching => self.do_searching(&buster),
            BusterState::Busting => self.do_bust(&buster),
            BusterState::MovingAwayFromGhost => self.do_away_ghost_move(&buster),
            BusterState::MovingToAsshole => self.do_to_asshole_move(&buster),
            BusterState::Stunning => self.do_stun(&mut buster),
            BusterState::MovingToHome => self.do_go_home(),
            BusterState::MovingToGhost => self.do_to_ghost_move(&buster),
            BusterState::Releasing => self.do_release(),
            BusterState::Stunned => self.do_searching(&buster), // placeholder
            BusterState::MovingToCamp => self.do_searching(&buster), //this works because moving is set
            BusterState::Camping => self.do_searching(&buster),
            _ => unreachable!(),
        };
        result.push_str(match &buster.state {
            BusterState::Idle => " Doing fuck all",
            BusterState::Searching => " Where the white women",
            BusterState::Busting => " I'm gonna bust",
            BusterState::MovingToHome => " Taking you home",
            BusterState::MovingAwayFromGhost => " That's my purse!",
            BusterState::MovingToAsshole => " Moving to asshole",
            BusterState::Stunning => " An electric bust",
            BusterState::MovingToGhost => " Bring me that ass",
            BusterState::Releasing => " Go to the pokeball",
            //TODO make more funnies
            _ => "",
        });
        return result;
    }
    fn do_searching(&self, buster: &Buster) -> String {
        return format!("MOVE {}", buster.movement_target.to_string());
    }
    fn do_bust(&self, buster: &Buster) -> String {
        return format!(
            "BUST {}",
            buster
                .busting_target_ref
                .as_ref()
                .unwrap()
                .borrow()
                .entity_id
        );
    }
    fn do_stun(&self, buster: &mut Buster) -> String {
        buster.stun_timer = STUN_COOLDOWN;
        return format!(
            "STUN {}",
            buster
                .stunning_target_ref
                .as_ref()
                .unwrap()
                .borrow()
                .entity_id
        );
    }
    fn do_away_ghost_move(&self, buster: &Buster) -> String {
        let ghost_coords = &buster.busting_target_ref.as_ref().unwrap().borrow().coords;

        let dx = ghost_coords.x - buster.coords.x;
        let dy = ghost_coords.y - buster.coords.y;
        if buster.coords.distance_to(&Coord { x: dy, y: dx }) < 300 {
            return format!("MOVE 0 0");
        }
        // Now calculate the reflection by subtracting the vector from the buster's position
        let reflected_x = buster.coords.x - dx;
        let reflected_y = buster.coords.y - dy;

        format!("MOVE {} {}", reflected_x, reflected_y)
    }
    fn do_to_ghost_move(&self, buster: &Buster) -> String {
        return format!(
            "MOVE {}",
            buster
                .busting_target_ref
                .as_ref()
                .unwrap()
                .borrow()
                .coords
                .to_string()
        );
    }
    fn do_to_asshole_move(&self, buster: &Buster) -> String {
        return format!(
            "MOVE {}",
            buster
                .stunning_target_ref
                .as_ref()
                .unwrap()
                .borrow()
                .coords
                .to_string()
        );
    }
    fn do_go_home(&self) -> String {
        return format!(
            "MOVE {}",
            if self.upper_left_home {
                Coord::new(0, 0).to_string()
            } else {
                Coord::new(MAP_RIGHT_WALL, MAP_BOTTOM_WALL).to_string()
            }
        );
    }
    fn do_release(&self) -> String {
        // buster.state = BusterState::Searching;
        return String::from("RELEASE");
    }
}

impl StateMachine {
    //state transitions and tests
    fn state_slide(&self, mut buster_refmut: std::cell::RefMut<'_, Buster>) -> () {
        while self.should_transition(&*buster_refmut) {
            eprintln!(
                "{} transitioned from {:?}",
                buster_refmut.entity_id, buster_refmut.state
            );
            self.do_transition(&mut *buster_refmut);
        }
    }

    fn should_transition(&self, buster: &Buster) -> bool {
        match buster.state {
            BusterState::Idle => true,
            BusterState::Searching => self.searching_test(buster),
            BusterState::Busting => self.busting_test(buster),
            BusterState::MovingToHome => self.house_move_test(buster),
            BusterState::MovingAwayFromGhost => self.ghost_moving_away_test(buster),
            BusterState::MovingToAsshole => self.asshole_moving_test(buster),
            BusterState::Stunning => self.stunning_test(buster),
            BusterState::MovingToGhost => self.ghost_moving_test(buster),
            BusterState::Releasing => self.release_test(buster),
            BusterState::Stunned => self.stunned_test(buster),
            BusterState::Camping => self.camping_test(buster),
            BusterState::MovingToCamp => self.camp_moving_test(buster),
            _ => false,
        }
    }
    fn camping_test(&self, buster: &Buster) -> bool {
        let stunnable_asshole_in_radius_with_ghost = self.bad_busters.iter().any(|asshole| {
            asshole
                .borrow()
                .coords
                .is_in_radius(&buster.coords, LINE_OF_SIGHT_RADIUS)
                && !matches!(asshole.borrow().state, BusterState::DeathMarked)
                && buster.can_stun()
                && asshole.borrow().has_ghost
        });
        let ghost_in_radius = self.all_ghosts.iter().any(|ghost| {
            ghost
                .borrow()
                .coords
                .is_in_radius(&buster.coords, LINE_OF_SIGHT_RADIUS)
        });
        let asking_for_help = !self.global_ping_stack.is_empty();
        stunnable_asshole_in_radius_with_ghost || ghost_in_radius || asking_for_help
    }
    fn camp_moving_test(&self, buster: &Buster) -> bool {
        let ghost_in_radius = self.all_ghosts.iter().any(|ghost| {
            ghost
                .borrow()
                .coords
                .is_in_radius(&buster.coords, LINE_OF_SIGHT_RADIUS)
        });
        let stunnable_asshole_in_radius_with_ghost = self.bad_busters.iter().any(|asshole| {
            asshole
                .borrow()
                .coords
                .is_in_radius(&buster.coords, LINE_OF_SIGHT_RADIUS)
                && !matches!(asshole.borrow().state, BusterState::DeathMarked)
                && buster.can_stun()
                && asshole.borrow().has_ghost
        });
        let made_it_to_camp = buster.coords.eq(&buster.movement_target);
        let asking_for_help = !self.global_ping_stack.is_empty();
        stunnable_asshole_in_radius_with_ghost
            || made_it_to_camp
            || ghost_in_radius
            || asking_for_help
    }
    fn stunned_test(&self, buster: &Buster) -> bool {
        !buster.is_stunned
    }
    fn searching_test(&self, buster: &Buster) -> bool {
        let ghost_in_radius = self.all_ghosts.iter().any(|ghost| {
            ghost
                .borrow()
                .coords
                .is_in_radius(&buster.coords, LINE_OF_SIGHT_RADIUS)
                && self.ghost_difficulty_test(&ghost.borrow())
        });
        let stunnable_asshole_in_radius = self.bad_busters.iter().any(|asshole| {
            asshole
                .borrow()
                .coords
                .is_in_radius(&buster.coords, LINE_OF_SIGHT_RADIUS)
                && !matches!(asshole.borrow().state, BusterState::DeathMarked)
                && buster.can_stun()
                && !asshole.borrow().is_stunned
        });
        let camping_time = self.turn_count >= START_CAMPING_TURN;
        let made_it_to_target = buster.coords.eq(&buster.movement_target);
        let asking_for_help =
            (!self.global_ping_stack.is_empty() && self.turn_count > TURNS_UNTIL_GLOBAL_PICKS);

        stunnable_asshole_in_radius
            || ghost_in_radius
            || made_it_to_target
            || asking_for_help
            || camping_time
    }
    fn asshole_moving_test(&self, buster: &Buster) -> bool {
        if let Some(asshole) = &buster.stunning_target_ref {
            //target exists, determine if in range
            asshole.borrow().is_stunnable(buster)
        } else {
            //target no longer exists
            true
        }
    }
    fn ghost_moving_away_test(&self, buster: &Buster) -> bool {
        if let Some(ghost) = &buster.busting_target_ref {
            //target exists and range is determined
            !ghost.borrow().is_too_close(buster)
        } else {
            //target no longer exists
            true
        }
    }
    fn ghost_moving_test(&self, buster: &Buster) -> bool {
        if let Some(_) = self.find_best_asshole_in_los(buster) {
            true
        } else if let Some(ghost) = &buster.busting_target_ref {
            //target exists and range is determined
            if let Some(best_ghost) = self.find_best_ghost_in_los(buster) {
                if best_ghost.ne(&ghost) && best_ghost.borrow().stamina <= ghost.borrow().stamina {
                    return true;
                }
            }
            buster
                .coords
                .is_in_radius(&ghost.borrow().coords, OUTER_BUSTING_RADIUS) //encapsulate case that we are too close
        } else {
            //target no longer exists
            true
        }
    }
    fn busting_test(&self, buster: &Buster) -> bool {
        return if let Some(_) = self.find_best_asshole_in_los(buster) {
            true
        } else {
            buster.busting_target_ref.is_none()
        };
    }
    fn stunning_test(&self, buster: &Buster) -> bool {
        if let Some(asshole) = &buster.stunning_target_ref {
            asshole.borrow().is_stunned
        } else {
            !buster.can_stun()
        }
    }
    fn house_move_test(&self, buster: &Buster) -> bool {
        let house_coords = if self.upper_left_home {
            Coord::new(0, 0)
        } else {
            Coord::new(MAP_RIGHT_WALL, MAP_BOTTOM_WALL)
        };
        //at house or lost ghost
        return house_coords.is_in_radius(&buster.coords, BASE_RELEASE_RADIUS) || !buster.has_ghost;
    }
    fn release_test(&self, buster: &Buster) -> bool {
        return !buster.has_ghost;
    }
}

impl StateMachine {
    //how to transition
    fn do_transition(&self, buster: &mut Buster) -> () {
        match buster.state {
            BusterState::Idle => self.do_idle_transition(buster),
            BusterState::Searching => self.do_searching_transition(buster),
            BusterState::Busting => self.do_busting_transition(buster),
            BusterState::MovingToHome => self.do_home_move_transition(buster),
            BusterState::MovingAwayFromGhost => self.do_ghost_away_move_transition(buster),
            BusterState::MovingToAsshole => self.do_asshole_move_transition(buster),
            BusterState::Stunning => self.do_stunning_transition(buster),
            BusterState::MovingToGhost => self.do_ghost_move_transition(buster),
            BusterState::Releasing => self.do_release_transition(buster),
            BusterState::Stunned => self.do_idle_transition(buster),
            BusterState::MovingToCamp => self.do_camp_move_transition(buster),
            BusterState::Camping => self.do_camping_transition(buster),
            _ => unreachable!(),
        }
    }
    fn do_camping_transition(&self, buster: &mut Buster) -> () {
        buster.state = if let Some(ghost_in_radius) = self.all_ghosts.iter().find(|ghost| {
            ghost
                .borrow()
                .coords
                .is_in_radius(&buster.coords, LINE_OF_SIGHT_RADIUS)
        }) {
            buster.busting_target_ref = Some(Rc::clone(ghost_in_radius));
            BusterState::MovingToGhost
        } else if let Some(stunnable_asshole_in_radius_with_ghost) =
            self.bad_busters.iter().find(|asshole| {
                asshole
                    .borrow()
                    .coords
                    .is_in_radius(&buster.coords, LINE_OF_SIGHT_RADIUS)
                    && !matches!(asshole.borrow().state, BusterState::DeathMarked)
                    && buster.can_stun()
                    && asshole.borrow().has_ghost
            })
        {
            self.set_asshole_target(stunnable_asshole_in_radius_with_ghost, buster);
            BusterState::MovingToAsshole
        } else if let Some(ghost_being_targetted) = self.steal_target_ghost_reference(buster) {
            buster.busting_target_ref = Some(ghost_being_targetted);
            BusterState::MovingToGhost
        } else {
            unreachable!()
        }
    }
    fn do_camp_move_transition(&self, buster: &mut Buster) -> () {
        buster.state = if buster.coords.eq(&buster.movement_target) {
            BusterState::Camping
        } else if let Some(ghost_in_radius) = self.all_ghosts.iter().find(|ghost| {
            ghost
                .borrow()
                .coords
                .is_in_radius(&buster.coords, LINE_OF_SIGHT_RADIUS)
        }) {
            buster.busting_target_ref = Some(Rc::clone(ghost_in_radius));
            BusterState::MovingToGhost
        } else if let Some(stunnable_asshole_in_radius_with_ghost) =
            self.bad_busters.iter().find(|asshole| {
                asshole
                    .borrow()
                    .coords
                    .is_in_radius(&buster.coords, LINE_OF_SIGHT_RADIUS)
                    && !matches!(asshole.borrow().state, BusterState::DeathMarked)
                    && buster.can_stun()
                    && asshole.borrow().has_ghost
            })
        {
            self.set_asshole_target(stunnable_asshole_in_radius_with_ghost, buster);
            BusterState::MovingToAsshole
        } else if let Some(ghost_being_targetted) = self.steal_target_ghost_reference(buster) {
            buster.busting_target_ref = Some(ghost_being_targetted);
            BusterState::MovingToGhost
        } else {
            unreachable!()
        }
    }
    fn do_idle_transition(&self, buster: &mut Buster) -> () {
        buster.state = BusterState::Searching;
    }

    fn do_ghost_move_transition(&self, buster: &mut Buster) -> () {
        if let Some(preferable_ghost) = self.find_best_ghost_in_los(buster) {
            // Check if this new preferable ghost is different than the current target
            if buster
                .busting_target_ref
                .as_ref()
                .map_or(true, |current_target| {
                    !Rc::ptr_eq(current_target, &preferable_ghost)
                })
            {
                // If it's different, update the target to the new preferable ghost
                buster.busting_target_ref = Some(Rc::clone(preferable_ghost));
                return;
            }
        }
        buster.state = if let Some(asshole) = self.find_best_asshole_in_los(buster) {
            self.set_asshole_target(&asshole, buster);
            BusterState::MovingToAsshole
        } else if let Some(ghost) = &buster.busting_target_ref {
            //target exists and range is determined
            // let best_ghost = self.find_best_ghost_in_LOS(buster).unwrap();
            if ghost.borrow().is_too_close(buster) {
                //too close to ghost
                BusterState::MovingAwayFromGhost
            } else {
                //right distance
                BusterState::Busting
            }
        } else {
            //target no longer exists
            BusterState::Searching
        }
    }
    fn do_asshole_move_transition(&self, buster: &mut Buster) -> () {
        buster.state = if let Some(asshole) = &buster.stunning_target_ref {
            BusterState::Stunning
        } else {
            BusterState::Searching
        }
    }
    fn do_ghost_away_move_transition(&self, buster: &mut Buster) -> () {
        if let Some(preferable_ghost) = self.find_best_ghost_in_los(buster) {
            // Check if this new preferable ghost is different than the current target
            if buster
                .busting_target_ref
                .as_ref()
                .map_or(true, |current_target| {
                    !Rc::ptr_eq(current_target, &preferable_ghost)
                })
            {
                // If it's different, update the target to the new preferable ghost
                buster.busting_target_ref = Some(preferable_ghost.clone());
            }
        }

        buster.state = if let Some(asshole) = self.find_best_asshole_in_los(buster) {
            self.set_asshole_target(&asshole, buster);
            BusterState::MovingToAsshole
        } else if let Some(ghost) = &buster.busting_target_ref {
            //target exists
            if ghost.borrow().is_bustable(buster) {
                //moved in radius
                BusterState::Busting
            } else {
                //too far away moving back
                BusterState::MovingToGhost
            }
        } else {
            //target no longer exists
            BusterState::Searching
        }
    }
    fn do_searching_transition(&self, buster: &mut Buster) -> () {
        if self.turn_count >= START_CAMPING_TURN {
            buster.state = BusterState::MovingToCamp;
        } else if buster.coords.eq(&(*buster).movement_target) {
            self.set_new_buster_movement_coords(buster);
        } else if let Some(asshole) = self.find_best_asshole_in_los(buster) {
            buster.state = BusterState::MovingToAsshole;
            self.set_asshole_target(&asshole, buster);
        } else if let Some(_) = self.find_ghost_in_radius_of_buster(buster) {
            buster.state = BusterState::MovingToGhost;
            buster.busting_target_ref = self.find_best_ghost_in_los(buster).cloned();
        } else if let Some(ghost_being_targetted) = self.steal_target_ghost_reference(buster) {
            buster.state = BusterState::MovingToGhost;
            buster.busting_target_ref = Some(ghost_being_targetted);
        }
    }
    fn do_busting_transition(&self, buster: &mut Buster) -> () {
        buster.state = if buster.has_ghost {
            BusterState::MovingToHome
        } else {
            if let Some(asshole) = self.find_best_asshole_in_los(buster) {
                self.set_asshole_target(&asshole, buster);
                BusterState::MovingToAsshole
            } else {
                BusterState::Searching
            }
        };
    }
    fn do_stunning_transition(&self, buster: &mut Buster) -> () {
        buster.state = if !buster.can_stun() {
            BusterState::Searching
        } else if let Some(asshole) = &mut buster.stunning_target_ref {
            //successfully stunned
            asshole.borrow_mut().state = BusterState::NotTargetted;
            buster.stunning_target_ref = None;
            BusterState::Searching
        } else {
            // he ran off?
            BusterState::MovingToAsshole
        };
    }
    fn do_home_move_transition(&self, buster: &mut Buster) -> () {
        buster.state = if buster.has_ghost {
            BusterState::Releasing
        } else {
            BusterState::Searching
        };
    }
    fn do_release_transition(&self, buster: &mut Buster) -> () {
        buster.state = BusterState::Searching;
    }
}

impl StateMachine {
    //ticks
    fn update_tick(
        &mut self,
        new_ghosts: Vec<Rc<RefCell<Ghost>>>,
        new_good: Vec<Rc<RefCell<Buster>>>,
        new_evil: Vec<Rc<RefCell<Buster>>>,
    ) -> () {
        self.update_ghosts(new_ghosts);
        if self.good_busters.is_empty() {
            self.good_busters = new_good;
            self.throw_and_assign_coords(false);
            self.bad_busters = new_evil;
        } else {
            self.update_good(new_good);
            self.update_evil(new_evil);
        }
        self.do_global_ping();
        self.turn_count += 1;
        if self.turn_count == START_CAMPING_TURN {
            self.throw_and_assign_coords(true);
        }
        eprintln!("global_stack_size: {}", self.global_ping_stack.len());
        eprintln!("turn count: {}", self.turn_count);
    }
    fn player_tick(&mut self, player_ind: usize) -> String {
        let player_rc = &self.good_busters[player_ind];
        {
            let mut player = player_rc.as_ref().borrow_mut();
            if player.is_stunned {
                player.state = BusterState::Stunned
            } else {
                self.state_slide(player);
            }
        }
        {
            let mut player = player_rc.as_ref().borrow_mut();
            player.tick();
        }
        let player = player_rc.as_ref().borrow_mut();
        let result = self.do_state(player);
        result
    }
}

fn main() {
    let mut input_line = String::new();
    io::stdin().read_line(&mut input_line).unwrap();
    let busters_per_player = parse_input!(input_line, i32); // the amount of busters you control
    let mut input_line = String::new();
    io::stdin().read_line(&mut input_line).unwrap();
    let ghost_count = parse_input!(input_line, i32); // the amount of ghosts on the map
    let mut input_line = String::new();
    io::stdin().read_line(&mut input_line).unwrap();
    let my_team_id = parse_input!(input_line, i32); // if this is 0, your base is on the top left of the map, if it is one, on the bottom right
    let mut game_machine = StateMachine::new(my_team_id);
    // game loop
    loop {
        let mut input_line = String::new();
        io::stdin().read_line(&mut input_line).unwrap();
        let entities = parse_input!(input_line, i32); // the number of busters and ghosts visible to you

        let mut ghost_tick_vec = Vec::new();
        let mut good_tick_vec = Vec::new();
        let mut evil_tick_vec = Vec::new();

        for i in 0..entities as usize {
            let mut input_line = String::new();
            io::stdin().read_line(&mut input_line).unwrap();
            let inputs = input_line.split(" ").collect::<Vec<_>>();
            let entity_id = parse_input!(inputs[0], i32); // buster id or ghost id
            let x = parse_input!(inputs[1], i32);
            let y = parse_input!(inputs[2], i32); // position of this buster / ghost
            let entity_type = parse_input!(inputs[3], i32); // the team id if it is a buster, -1 if it is a ghost.
            let state = parse_input!(inputs[4], i32); // For busters: 0=idle, 1=carrying a ghost.
            let value = parse_input!(inputs[5], i32); // For busters: Ghost id being carried. For ghosts: number of busters attempting to trap this ghost.
            match entity_type {
                -1 => {
                    let new_entity = Ghost::new(entity_id, x, y, value, state);
                    ghost_tick_vec.push(Rc::clone(&new_entity));
                }
                bust if bust == my_team_id => {
                    let new_entity = Buster::new(entity_id, x, y, state == 1, state == 2);
                    good_tick_vec.push(Rc::clone(&new_entity))
                }
                _ => {
                    let new_entity = Buster::new_opponent(entity_id, x, y, state == 1, state == 2);
                    evil_tick_vec.push(Rc::clone(&new_entity))
                }
            }
        }
        game_machine.update_tick(ghost_tick_vec, good_tick_vec, evil_tick_vec);

        for i in 0..busters_per_player as usize {
            let result = game_machine.player_tick(i);
            println!("{}", result);
        }
    }
}