all:
	make run
run:
	cargo run /dev/ttyUSB0 115200 1 VDC 3_3
