import os
import signal
from pygdbmi.gdbcontroller import GdbController

DEFAULT_KERNEL_PATH = "target/riscv64gc-unknown-none-elf/release/kernel"
GDB_PORT_FILE = ".gdb-port"


class GDBSession:
    def __init__(self):
        self._gdb: GdbController | None = None

    @property
    def connected(self) -> bool:
        return self._gdb is not None

    def _require_gdb(self) -> GdbController:
        if self._gdb is None:
            raise RuntimeError("GDB is not running. Call gdb_connect first.")
        return self._gdb

    def start(self, gdb_path: str = "gdb") -> list[dict]:
        if self._gdb is not None:
            raise RuntimeError("GDB is already running.")
        self._gdb = GdbController(
            command=[gdb_path, "--interpreter=mi3", "--quiet"],
        )
        return self._gdb.get_gdb_response(timeout_sec=5)

    def connect_remote(
        self, port: int, kernel_path: str = DEFAULT_KERNEL_PATH
    ) -> list[dict]:
        gdb = self._require_gdb()
        responses = []
        for cmd in [
            "set architecture riscv:rv64",
            "set pagination off",
            "set auto-load safe-path /",
            f"file {kernel_path}",
            f"target remote :{port}",
        ]:
            responses.extend(gdb.write(cmd, timeout_sec=10))
        return responses

    def execute_mi(self, command: str, timeout_sec: int = 30) -> list[dict]:
        gdb = self._require_gdb()
        return gdb.write(command, timeout_sec=timeout_sec)

    def execute_cli(self, command: str, timeout_sec: int = 30) -> list[dict]:
        gdb = self._require_gdb()
        return gdb.write(
            f"-interpreter-exec console \"{command}\"", timeout_sec=timeout_sec
        )

    def interrupt(self):
        gdb = self._require_gdb()
        if gdb.gdb_process and gdb.gdb_process.pid:
            os.kill(gdb.gdb_process.pid, signal.SIGINT)

    def stop(self):
        if self._gdb is not None:
            try:
                self._gdb.exit()
            except Exception:
                pass
            self._gdb = None

    @staticmethod
    def read_gdb_port() -> int | None:
        try:
            with open(GDB_PORT_FILE) as f:
                return int(f.read().strip())
        except (FileNotFoundError, ValueError):
            return None
