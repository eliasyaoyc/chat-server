mod config;
mod errors;
pub mod svc;
pub mod serve;
mod worker;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
