use crate::{
    types::Address,
    utils::{secret_key_to_address, unused_port},
};
use k256::{ecdsa::SigningKey, SecretKey as K256SecretKey};
use std::{
    io::{BufRead, BufReader},
    process::{Child, Command},
    time::{Duration, Instant},
};

/// How long we will wait for anvil to indicate that it is ready.
const ANVIL_STARTUP_TIMEOUT_MILLIS: u64 = 5_000;

/// An anvil CLI instance. Will close the instance when dropped.
///
/// Construct this using [`Anvil`](crate::utils::Anvil)
pub struct AnvilInstance {
    pid: Child,
    private_keys: Vec<K256SecretKey>,
    addresses: Vec<Address>,
    port: u16,
}

impl AnvilInstance {
    /// Returns the private keys used to instantiate this instance
    pub fn keys(&self) -> &[K256SecretKey] {
        &self.private_keys
    }

    /// Returns the addresses used to instantiate this instance
    pub fn addresses(&self) -> &[Address] {
        &self.addresses
    }

    /// Returns the port of this instance
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Returns the HTTP endpoint of this instance
    pub fn endpoint(&self) -> String {
        format!("http://localhost:{}", self.port)
    }

    /// Returns the Websocket endpoint of this instance
    pub fn ws_endpoint(&self) -> String {
        format!("ws://localhost:{}", self.port)
    }
}

impl Drop for AnvilInstance {
    fn drop(&mut self) {
        self.pid.kill().expect("could not kill anvil");
    }
}

/// Builder for launching `anvil`.
///
/// # Panics
///
/// If `spawn` is called without `anvil` being available in the user's $PATH
///
/// # Example
///
/// ```no_run
/// use ethers_core::utils::Anvil;
///
/// let port = 8545u16;
/// let url = format!("http://localhost:{}", port).to_string();
///
/// let anvil = Anvil::new()
///     .port(port)
///     .mnemonic("abstract vacuum mammal awkward pudding scene penalty purchase dinner depart evoke puzzle")
///     .spawn();
///
/// drop(anvil); // this will kill the instance
/// ```
#[derive(Debug, Clone, Default)]
pub struct Anvil {
    port: Option<u16>,
    block_time: Option<u64>,
    mnemonic: Option<String>,
    fork: Option<String>,
    args: Vec<String>,
}

impl Anvil {
    /// Creates an empty Anvil builder.
    /// The default port is 8545. The mnemonic is chosen randomly.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the port which will be used when the `anvil` instance is launched.
    #[must_use]
    pub fn port<T: Into<u16>>(mut self, port: T) -> Self {
        self.port = Some(port.into());
        self
    }

    /// Sets the mnemonic which will be used when the `anvil` instance is launched.
    #[must_use]
    pub fn mnemonic<T: Into<String>>(mut self, mnemonic: T) -> Self {
        self.mnemonic = Some(mnemonic.into());
        self
    }

    /// Sets the block-time which will be used when the `anvil` instance is launched.
    #[must_use]
    pub fn block_time<T: Into<u64>>(mut self, block_time: T) -> Self {
        self.block_time = Some(block_time.into());
        self
    }

    /// Sets the `fork` argument to fork from another currently running Ethereum client
    /// at a given block. Input should be the HTTP location and port of the other client,
    /// e.g. `http://localhost:8545`. You can optionally specify the block to fork from
    /// using an @ sign: `http://localhost:8545@1599200`
    #[must_use]
    pub fn fork<T: Into<String>>(mut self, fork: T) -> Self {
        self.fork = Some(fork.into());
        self
    }

    /// Adds an argument to pass to the `anvil`.
    #[must_use]
    pub fn arg<T: Into<String>>(mut self, arg: T) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Adds multiple arguments to pass to the `anvil`.
    #[must_use]
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for arg in args {
            self = self.arg(arg);
        }
        self
    }

    /// Consumes the builder and spawns `anvil` with stdout redirected
    /// to /dev/null.
    pub fn spawn(self) -> AnvilInstance {
        let mut cmd = Command::new("anvil");
        cmd.stdout(std::process::Stdio::piped());
        let port = if let Some(port) = self.port { port } else { unused_port() };
        cmd.arg("-p").arg(port.to_string());

        if let Some(mnemonic) = self.mnemonic {
            cmd.arg("-m").arg(mnemonic);
        }

        if let Some(block_time) = self.block_time {
            cmd.arg("-b").arg(block_time.to_string());
        }

        if let Some(fork) = self.fork {
            cmd.arg("-f").arg(fork);
        }

        cmd.args(self.args);

        let mut child = cmd.spawn().expect("couldnt start anvil");

        let stdout = child.stdout.expect("Unable to get stdout for anvil child process");

        let start = Instant::now();
        let mut reader = BufReader::new(stdout);

        let mut private_keys = Vec::new();
        let mut addresses = Vec::new();
        let mut is_private_key = false;
        loop {
            if start + Duration::from_millis(ANVIL_STARTUP_TIMEOUT_MILLIS) <= Instant::now() {
                panic!("Timed out waiting for anvil to start. Is anvil installed?")
            }

            let mut line = String::new();
            reader.read_line(&mut line).expect("Failed to read line from anvil process");
            if line.contains("Listening on") {
                break
            }

            if line.starts_with("Private Keys") {
                is_private_key = true;
            }

            if is_private_key && line.starts_with('(') {
                let key_str = &line[6..line.len() - 1];
                let key_hex = hex::decode(key_str).expect("could not parse as hex");
                let key = K256SecretKey::from_be_bytes(&key_hex).expect("did not get private key");
                addresses.push(secret_key_to_address(&SigningKey::from(&key)));
                private_keys.push(key);
            }
        }

        child.stdout = Some(reader.into_inner());

        AnvilInstance { pid: child, private_keys, addresses, port }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn can_launch_anvil() {
        let _ = Anvil::new().spawn();
    }
}
