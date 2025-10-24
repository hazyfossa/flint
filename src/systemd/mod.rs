pub mod dbus {
    // Varlink login api does not support seat events
    pub mod logind;
}

pub mod varlink {
    pub mod userdb;
}
