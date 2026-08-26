use rrule::RRuleSet;
use std::str::FromStr;
fn main() {
    let r = RRuleSet::from_str("RRULE:FREQ=DAILY");
    println!("{:?}", r);
}
