


release:
	mkdir release
	GOOS=darwin GOARCH=arm64 go build -o release/controlrake_darwin ./src/main/...
	GOOS=windows GOARCH=amd64 go build -o release/controlrake_windows.exe ./src/main/...
	GOOS=linux GOARCH=amd64 go build -o release/controlrake_linux ./src/main/...
	cp config.yml release/
	mkdir release/sounds release/static
	cp -r sounds release/
	cp -r static release/
	cp LICENSE release/
	cp README.md release/
	(pushd release; tar -czf ../controlrake.tgz *)