use moon_process::SignalType;

mod signal_type {
    use super::*;

    #[test]
    fn maps_to_unix_codes() {
        assert_eq!(SignalType::Interrupt.get_code(), 2);
        assert_eq!(SignalType::Quit.get_code(), 3);
        assert_eq!(SignalType::Kill.get_code(), 9);
        assert_eq!(SignalType::Terminate.get_code(), 15);
    }
}
