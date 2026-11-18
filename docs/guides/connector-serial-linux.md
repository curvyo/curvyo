# The connector's serial port on Linux

`curvyo-connector-server --serial <port> [--baud <n>]` talks to a GRBL controller over USB serial
(`specs/0127-connector-grbl-serial`, architect addendum C10 in `specs/0064-machine-connector/adrs.md`).
This page says what the computer next to the laser needs. Windows and macOS are not covered yet.

## Port access

- The port is usually `/dev/ttyUSB0` (CH340 and FTDI adapters) or `/dev/ttyACM0` (Arduino Uno and
  native USB boards). A stable name is under `/dev/serial/by-id/`.
- The user that runs the connector must be in the group that owns the port: `dialout` on Debian,
  Ubuntu and Fedora, `uucp` on Arch. `ls -l /dev/ttyUSB0` shows it. After
  `sudo usermod -aG <group> <user>` log in again.
- Alternatively a `udev` rule grants access, for example in
  `/etc/udev/rules.d/70-laser.rules`:

  ```text
  SUBSYSTEM=="tty", ATTRS{idVendor}=="1a86", ATTRS{idProduct}=="7523", TAG+="uaccess"
  ```

  (`TAG+="uaccess"` for the logged-in user, or `GROUP="<group>", MODE="0660"` for a service user;
  `udevadm info /dev/ttyUSB0` shows the vendor and product ids).
- When the open is refused, the connector's terminal names the port, its group and whether the
  user is a member: "Cannot open /dev/ttyUSB0: permission denied (group uucp, this user is not a
  member)".

## ModemManager

ModemManager may probe new `ttyACM` and `ttyUSB` devices with modem commands, which disturbs a
controller. The connector prints one line at start when ModemManager runs. It cannot stop it
(ModemManager runs as root), but its own exclusive open keeps other non-root programs out. An
answer the connector did not ask for stops the job (G28) when no line is waiting for its
answer; while lines are in flight it is caught only when GRBL reports its buffer (`$10` with the
`Bf:` field, value 2 added), otherwise it cannot be told apart from GRBL's own. So make ModemManager
leave the laser alone, with this line in the `udev` rule above:

```text
ENV{ID_MM_DEVICE_IGNORE}="1"
```

or stop the service when the computer has no modem: `sudo systemctl disable --now ModemManager`.

## What the connector does on the port

- Every open starts with GRBL's soft reset, so the laser is off whatever an earlier program left.
- The port is opened exclusively; a second connector or terminal program gets "in use".
- A missing port, or one that does not answer as GRBL, is tried again every 2 s; the connector
  never exits for it.
- SIGINT and SIGTERM stop a running job (feed hold, reset, `M5`, `M9`) before the exit. `kill -9`,
  SIGHUP and power loss cannot be handled; GRBL's own planner keeps moving for a moment after a
  pulled cable, which no software can undo.

## Without a laser

`cargo run -p curvyo-connector-io --example grbl_pty` starts a GRBL 1.1 simulator on a
pseudo-terminal and prints its path; `curvyo-connector-server --serial <that path>` runs against it
as against a laser.
