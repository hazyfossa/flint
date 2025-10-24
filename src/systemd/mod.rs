pub mod dbus {
    // Varlink login api does not support seat events
    pub mod login;
}

pub mod varlink {
    pub mod userdb;
}
