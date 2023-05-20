#[cfg(feature = "runtime")]
use mom::print_utils::MomOutput;

#[cfg(feature = "runtime")]
use mom::cli::exec;

#[cfg(feature = "runtime")]
fn main() {
    match exec() {
        Ok(_) => {}
        Err(e) => {
            eprint!("{}", e.to_string().mom_error());
            std::process::exit(1);
        }
    }
}
