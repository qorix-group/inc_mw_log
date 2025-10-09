// #[macro_export]
// macro_rules! info {
//     ($fmt:expr $(, $arg:expr )* $(,)?) => {{
//         let logger = $crate::custom_log::private_api::get_global_logger();
//         if logger.is_null() { return; }
//         $crate::custom_log::private_api::begin_and_push(logger, /*level*/ 2, $fmt, |stream| {
//             $(
//                 $crate::custom_log::private_api::push_arg(stream, $arg);
//             )*
//         });
//     }}
// }

#[macro_export]
macro_rules! log_info {
    // key => value pairs
    ($($key:expr => $val:expr),* $(,)?) => {{
        let mut values = Vec::<$crate::LogValue>::new();
        $(
            values.push($crate::to_log_value($val));
        )*
        $crate::send_record($crate::Level::Info, values);
    }};
    // single values without key
    ($($val:expr),+ $(,)?) => {{
        let mut values = Vec::<$crate::LogValue>::new();
        $(
            values.push($crate::to_log_value($val));
        )+
        $crate::send_record($crate::Level::Info, values);
    }};
}

#[macro_export]
macro_rules! log_debug {
    ($($key:expr => $val:expr),* $(,)?) => {{
        let mut values = Vec::<$crate::LogValue>::new();
        $(
            values.push($crate::to_log_value($val));
        )*
        $crate::log_record!($crate::Level::Debug, values);
    }};
}

#[macro_export]
macro_rules! log_record {
    ($level:expr, $values:expr) => {{
        let mut logger = mw_log_subscriber::MwLogger::global();
        logger.send_record($level, $values);
    }};
}
