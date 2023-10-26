use std::io;

macro_rules! parse_input {
    ($x:expr, $t:ident) => {
        $x.trim().parse::<$t>().unwrap()
    };
}
fn print_map(map: &Vec<Vec<i32>>) -> String {
    let mut result = String::from("+---[CODINGAME]---+\n");
    let mut push_result = |x: String| {
        result.push_str(&format!("{}\n", x));
    };
    let border_add = |input: String| format!("|{}|", input);
    let step_key = [
        ' ', '.', 'o', '+', '=', '*', 'B', 'O', 'X', '@', '%', '&', '#', '/', '^',
    ];
    let convert_steps = |cell: i32| -> char {
        if cell >= 0 {
            return step_key[(((cell % 15) + 15) % 15) as usize];
        } else {
            return match cell {
                -1 => 'S',
                -2 => 'E',
                _ => unreachable!(),
            };
        }
    };
    for row in map {
        let mut translated_row = String::new();
        row.iter()
            .for_each(|cell| translated_row.push(convert_steps(*cell)));
        push_result(border_add(translated_row));
    }
    result.push_str("+-----------------+");
    result
}

fn do_step(map: &mut Vec<Vec<i32>>, cursor: (i32, i32)) -> () {
    (*map)[cursor.1 as usize][cursor.0 as usize] += 1;
}

enum Direction {
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
}

fn main() {
    let cursor_start = (8, 4);
    let mut cursor = cursor_start.clone();

    let mut map = vec![vec![0; 17]; 9];

    let mut move_cursor = |direction: Direction, local_cur: (i32, i32)| -> (i32, i32) {
        let (dx, dy) = match direction {
            Direction::UpLeft => (-1, -1),
            Direction::UpRight => (1, -1),
            Direction::DownLeft => (-1, 1),
            Direction::DownRight => (1, 1),
        };

        let clamp = |val: i32, min: i32, max: i32| val.max(min).min(max);

        return (
            clamp(local_cur.0 + dx, 0, 16),
            clamp(local_cur.1 + dy, 0, 8),
        );
    };

    let mut input_line = String::new();
    io::stdin().read_line(&mut input_line).unwrap();

    let fingerprint = input_line.trim_matches('\n').to_string();
    for byte_str in fingerprint.split(":") {
        let byte_bins = format!("{:08b}", u8::from_str_radix(&byte_str, 16).unwrap());
        use std::str;
        let subs = byte_bins
            .as_bytes()
            .chunks(2)
            .rev()
            .map(|buf| unsafe { str::from_utf8_unchecked(buf) })
            .collect::<Vec<&str>>();

        for sub in subs.iter() {
            let direction = match *sub {
                "00" => 
                    Direction::UpLeft,
                "01" => Direction::UpRight,
                "10" => Direction::DownLeft,
                "11" => Direction::DownRight,
                _ => unreachable!(),
            };
            cursor = move_cursor(direction, cursor);
            do_step(&mut map, cursor);
        }
    }
    // Write an answer using println!("message...");
    // To debug: eprintln!("Debug message...");
    map[cursor_start.1 as usize][cursor_start.0 as usize] = -1;
    map[cursor.1 as usize][cursor.0 as usize] = -2;
    println!("{}", print_map(&map));
}
