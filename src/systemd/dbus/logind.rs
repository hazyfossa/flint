use zbus::proxy;

pub use DefinitionProxy as LoginD;
#[proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
pub trait Definition {
    fn activate_session(&self, session_id: &str) -> zbus::Result<()>;

    fn activate_session_on_seat(&self, session_id: &str, seat_id: &str) -> zbus::Result<()>;

    fn attach_device(&self, seat_id: &str, sysfs_path: &str, interactive: bool)
    -> zbus::Result<()>;

    fn can_halt(&self) -> zbus::Result<String>;

    fn can_hibernate(&self) -> zbus::Result<String>;

    fn can_hybrid_sleep(&self) -> zbus::Result<String>;

    fn can_power_off(&self) -> zbus::Result<String>;

    fn can_reboot(&self) -> zbus::Result<String>;

    fn can_reboot_parameter(&self) -> zbus::Result<String>;

    fn can_reboot_to_boot_loader_entry(&self) -> zbus::Result<String>;

    fn can_reboot_to_boot_loader_menu(&self) -> zbus::Result<String>;

    fn can_reboot_to_firmware_setup(&self) -> zbus::Result<String>;

    fn can_sleep(&self) -> zbus::Result<String>;

    fn can_suspend(&self) -> zbus::Result<String>;

    fn can_suspend_then_hibernate(&self) -> zbus::Result<String>;

    fn cancel_scheduled_shutdown(&self) -> zbus::Result<bool>;

    #[allow(clippy::too_many_arguments)]
    fn create_session(
        &self,
        uid: u32,
        pid: u32,
        service: &str,
        type_: &str,
        class: &str,
        desktop: &str,
        seat_id: &str,
        vtnr: u32,
        tty: &str,
        display: &str,
        remote: bool,
        remote_user: &str,
        remote_host: &str,
        properties: &[&(&str, &zbus::zvariant::Value<'_>)],
    ) -> zbus::Result<(
        String,
        zbus::zvariant::OwnedObjectPath,
        String,
        zbus::zvariant::OwnedFd,
        u32,
        String,
        u32,
        bool,
    )>;

    #[zbus(name = "CreateSessionWithPIDFD")]
    #[allow(clippy::too_many_arguments)]
    fn create_session_with_pidfd(
        &self,
        uid: u32,
        pidfd: zbus::zvariant::Fd<'_>,
        service: &str,
        type_: &str,
        class: &str,
        desktop: &str,
        seat_id: &str,
        vtnr: u32,
        tty: &str,
        display: &str,
        remote: bool,
        remote_user: &str,
        remote_host: &str,
        flags: u64,
        properties: &[&(&str, &zbus::zvariant::Value<'_>)],
    ) -> zbus::Result<(
        String,
        zbus::zvariant::OwnedObjectPath,
        String,
        zbus::zvariant::OwnedFd,
        u32,
        String,
        u32,
        bool,
    )>;

    fn flush_devices(&self, interactive: bool) -> zbus::Result<()>;

    fn get_seat(&self, seat_id: &str) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

    fn get_session(&self, session_id: &str) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

    #[zbus(name = "GetSessionByPID")]
    fn get_session_by_pid(&self, pid: u32) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

    fn get_user(&self, uid: u32) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

    #[zbus(name = "GetUserByPID")]
    fn get_user_by_pid(&self, pid: u32) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

    fn halt(&self, interactive: bool) -> zbus::Result<()>;

    fn halt_with_flags(&self, flags: u64) -> zbus::Result<()>;

    fn hibernate(&self, interactive: bool) -> zbus::Result<()>;

    fn hibernate_with_flags(&self, flags: u64) -> zbus::Result<()>;

    fn hybrid_sleep(&self, interactive: bool) -> zbus::Result<()>;

    fn hybrid_sleep_with_flags(&self, flags: u64) -> zbus::Result<()>;

    fn inhibit(
        &self,
        what: &str,
        who: &str,
        why: &str,
        mode: &str,
    ) -> zbus::Result<zbus::zvariant::OwnedFd>;

    fn kill_session(&self, session_id: &str, whom: &str, signal_number: i32) -> zbus::Result<()>;

    fn kill_user(&self, uid: u32, signal_number: i32) -> zbus::Result<()>;

    fn list_inhibitors(&self) -> zbus::Result<Vec<(String, String, String, String, u32, u32)>>;

    fn list_seats(&self) -> zbus::Result<Vec<(String, zbus::zvariant::OwnedObjectPath)>>;

    fn list_sessions(
        &self,
    ) -> zbus::Result<Vec<(String, u32, String, String, zbus::zvariant::OwnedObjectPath)>>;

    fn list_sessions_ex(
        &self,
    ) -> zbus::Result<
        Vec<(
            String,
            u32,
            String,
            String,
            u32,
            String,
            String,
            bool,
            u64,
            zbus::zvariant::OwnedObjectPath,
        )>,
    >;

    fn list_users(&self) -> zbus::Result<Vec<(u32, String, zbus::zvariant::OwnedObjectPath)>>;

    fn lock_session(&self, session_id: &str) -> zbus::Result<()>;

    fn lock_sessions(&self) -> zbus::Result<()>;

    fn power_off(&self, interactive: bool) -> zbus::Result<()>;

    fn power_off_with_flags(&self, flags: u64) -> zbus::Result<()>;

    fn reboot(&self, interactive: bool) -> zbus::Result<()>;

    fn reboot_with_flags(&self, flags: u64) -> zbus::Result<()>;

    fn release_session(&self, session_id: &str) -> zbus::Result<()>;

    fn schedule_shutdown(&self, type_: &str, usec: u64) -> zbus::Result<()>;

    fn set_reboot_parameter(&self, parameter: &str) -> zbus::Result<()>;

    fn set_reboot_to_boot_loader_entry(&self, boot_loader_entry: &str) -> zbus::Result<()>;

    fn set_reboot_to_boot_loader_menu(&self, timeout: u64) -> zbus::Result<()>;

    fn set_reboot_to_firmware_setup(&self, enable: bool) -> zbus::Result<()>;

    fn set_user_linger(&self, uid: u32, enable: bool, interactive: bool) -> zbus::Result<()>;

    fn set_wall_message(&self, wall_message: &str, enable: bool) -> zbus::Result<()>;

    fn sleep(&self, flags: u64) -> zbus::Result<()>;

    fn suspend(&self, interactive: bool) -> zbus::Result<()>;

    fn suspend_then_hibernate(&self, interactive: bool) -> zbus::Result<()>;

    fn suspend_then_hibernate_with_flags(&self, flags: u64) -> zbus::Result<()>;

    fn suspend_with_flags(&self, flags: u64) -> zbus::Result<()>;

    fn terminate_seat(&self, seat_id: &str) -> zbus::Result<()>;

    fn terminate_session(&self, session_id: &str) -> zbus::Result<()>;

    fn terminate_user(&self, uid: u32) -> zbus::Result<()>;

    fn unlock_session(&self, session_id: &str) -> zbus::Result<()>;

    fn unlock_sessions(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn prepare_for_shutdown(&self, start: bool) -> zbus::Result<()>;

    #[zbus(signal)]
    fn prepare_for_shutdown_with_metadata(
        &self,
        start: bool,
        metadata: std::collections::HashMap<&str, zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    fn prepare_for_sleep(&self, start: bool) -> zbus::Result<()>;

    #[zbus(signal)]
    fn seat_new(
        &self,
        seat_id: &str,
        object_path: zbus::zvariant::ObjectPath<'_>,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    fn seat_removed(
        &self,
        seat_id: &str,
        object_path: zbus::zvariant::ObjectPath<'_>,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    fn secure_attention_key(
        &self,
        seat_id: &str,
        object_path: zbus::zvariant::ObjectPath<'_>,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    fn session_new(
        &self,
        session_id: &str,
        object_path: zbus::zvariant::ObjectPath<'_>,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    fn session_removed(
        &self,
        session_id: &str,
        object_path: zbus::zvariant::ObjectPath<'_>,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    fn user_new(&self, uid: u32, object_path: zbus::zvariant::ObjectPath<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    fn user_removed(
        &self,
        uid: u32,
        object_path: zbus::zvariant::ObjectPath<'_>,
    ) -> zbus::Result<()>;

    #[zbus(property)]
    fn block_inhibited(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn block_weak_inhibited(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn boot_loader_entries(&self) -> zbus::Result<Vec<String>>;

    #[zbus(property)]
    fn delay_inhibited(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn designated_maintenance_time(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn docked(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn enable_wall_messages(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_enable_wall_messages(&self, value: bool) -> zbus::Result<()>;

    #[zbus(property)]
    fn handle_hibernate_key(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn handle_hibernate_key_long_press(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn handle_lid_switch(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn handle_lid_switch_docked(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn handle_lid_switch_external_power(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn handle_power_key(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn handle_power_key_long_press(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn handle_reboot_key(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn handle_reboot_key_long_press(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn handle_secure_attention_key(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn handle_suspend_key(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn handle_suspend_key_long_press(&self) -> zbus::Result<String>;

    #[zbus(property, name = "HoldoffTimeoutUSec")]
    fn holdoff_timeout_usec(&self) -> zbus::Result<u64>;

    #[zbus(property)]
    fn idle_action(&self) -> zbus::Result<String>;

    #[zbus(property, name = "IdleActionUSec")]
    fn idle_action_usec(&self) -> zbus::Result<u64>;

    #[zbus(property)]
    fn idle_hint(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn idle_since_hint(&self) -> zbus::Result<u64>;

    #[zbus(property)]
    fn idle_since_hint_monotonic(&self) -> zbus::Result<u64>;

    #[zbus(property, name = "InhibitDelayMaxUSec")]
    fn inhibit_delay_max_usec(&self) -> zbus::Result<u64>;

    #[zbus(property)]
    fn inhibitors_max(&self) -> zbus::Result<u64>;

    #[zbus(property)]
    fn kill_exclude_users(&self) -> zbus::Result<Vec<String>>;

    #[zbus(property)]
    fn kill_only_users(&self) -> zbus::Result<Vec<String>>;

    #[zbus(property)]
    fn kill_user_processes(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn lid_closed(&self) -> zbus::Result<bool>;

    #[zbus(property, name = "NAutoVTs")]
    fn nauto_vts(&self) -> zbus::Result<u32>;

    #[zbus(property, name = "NCurrentInhibitors")]
    fn ncurrent_inhibitors(&self) -> zbus::Result<u64>;

    #[zbus(property, name = "NCurrentSessions")]
    fn ncurrent_sessions(&self) -> zbus::Result<u64>;

    #[zbus(property)]
    fn on_external_power(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn preparing_for_shutdown(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn preparing_for_shutdown_with_metadata(
        &self,
    ) -> zbus::Result<std::collections::HashMap<String, zbus::zvariant::OwnedValue>>;

    #[zbus(property)]
    fn preparing_for_sleep(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn reboot_parameter(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn reboot_to_boot_loader_entry(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn reboot_to_boot_loader_menu(&self) -> zbus::Result<u64>;

    #[zbus(property)]
    fn reboot_to_firmware_setup(&self) -> zbus::Result<bool>;

    #[zbus(property, name = "RemoveIPC")]
    fn remove_ipc(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn runtime_directory_inodes_max(&self) -> zbus::Result<u64>;

    #[zbus(property)]
    fn runtime_directory_size(&self) -> zbus::Result<u64>;

    #[zbus(property)]
    fn scheduled_shutdown(&self) -> zbus::Result<(String, u64)>;

    #[zbus(property)]
    fn sessions_max(&self) -> zbus::Result<u64>;

    #[zbus(property)]
    fn sleep_operation(&self) -> zbus::Result<Vec<String>>;

    #[zbus(property, name = "StopIdleSessionUSec")]
    fn stop_idle_session_usec(&self) -> zbus::Result<u64>;

    #[zbus(property, name = "UserStopDelayUSec")]
    fn user_stop_delay_usec(&self) -> zbus::Result<u64>;

    #[zbus(property)]
    fn wall_message(&self) -> zbus::Result<String>;
}
