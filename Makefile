


release:
	GOOS=darwin GOARCH=arm64 go build -o controlrake_darwin ./src/main/...
	GOOS=windows GOARCH=amd64 go build -o controlrake_windows.exe ./src/main/...
	GOOS=linux GOARCH=amd64 go build -o controlrake_linux ./src/main/...
	mkdir release
	cp config.yml release/
	mv controlrake_darwin release/
	mv controlrake_windows.exe release/
	mv controlrake_linux release/
	mkdir release/sounds release/static
	cp -r sounds release/
	cp -r static release/
	cp LICENSE release/
	cp README.md release/
	(pushd release; tar -czf ../controlrake.tgz *)