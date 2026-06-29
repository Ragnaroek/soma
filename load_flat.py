import gdb


def load_flat_binary(filename, load_addr):
    # Read the binary file
    with open(filename, "rb") as f:
        data = f.read()

    # Write to memory (requires a live target with write access)
    inferior = gdb.selected_inferior()
    for i, byte in enumerate(data):
        inferior.write_memory(load_addr + i, bytes([byte]))

    # Set the program counter
    gdb.execute(f"set $pc = {load_addr}")
    print(f"Loaded {len(data)} bytes from {filename} to 0x{load_addr:x}")


class LoadFlatBinary(gdb.Command):
    def __init__(self):
        super(LoadFlatBinary, self).__init__("load-flat", gdb.COMMAND_USER)

    def invoke(self, arg, from_tty):
        args = arg.split()
        if len(args) != 2:
            print("Usage: load-flat <file> <load_address>")
            return

        filename = args[0]
        load_addr = int(args[1], 16)
        load_flat_binary(filename, load_addr)


LoadFlatBinary()
