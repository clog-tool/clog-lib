macro_rules! regex(
    ($s:expr) => (::regex::Regex::new($s).unwrap());
);
