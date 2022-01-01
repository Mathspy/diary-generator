use time::Month;

const MONTHS: &[Month] = &[
    Month::January,
    Month::February,
    Month::March,
    Month::April,
    Month::May,
    Month::June,
    Month::July,
    Month::August,
    Month::September,
    Month::October,
    Month::November,
    Month::December,
];

pub fn all() -> std::slice::Iter<'static, Month> {
    MONTHS.iter()
}
