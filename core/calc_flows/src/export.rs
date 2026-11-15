//! Prints every flow as JSON, for the front ends' drivers (`:flows_json`).

fn main() {
    print!("{}", calc_flows::to_json(&calc_flows::flows::all()));
}
