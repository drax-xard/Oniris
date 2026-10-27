CARGO_MANIFEST := core/Cargo.toml
CLI := cargo run --manifest-path $(CARGO_MANIFEST) -p oniris-cli --

.PHONY: all test fmt fmt-check clippy packs packs-wav noise procedural pack render android-native android-packs android beta clean

all: fmt-check clippy test

test:
	cargo test --manifest-path $(CARGO_MANIFEST) --workspace

fmt:
	cargo fmt --manifest-path $(CARGO_MANIFEST) --all

fmt-check:
	cargo fmt --manifest-path $(CARGO_MANIFEST) --all --check

clippy:
	cargo clippy --manifest-path $(CARGO_MANIFEST) --workspace --all-targets -- -D warnings

packs:
	$(CLI) gen-test-packs packs

packs-wav:
	$(CLI) gen-test-packs packs-wav --format wav

noise:
	$(CLI) gen-noise-pack packs

procedural:
	$(CLI) gen-procedural-packs packs

pack: packs
	$(CLI) pack packs/test-rain test-rain.onirispack
	$(CLI) verify-pack test-rain.onirispack

render:
	$(CLI) render --pack packs/test-rain --pack packs/test-fire --minutes 10 --out oniris-render.wav

android-native:
	cd core && cargo ndk \
		-t arm64-v8a -t armeabi-v7a -t x86_64 \
		-o ../android/app/src/main/jniLibs \
		build -p oniris-jni --release

android-packs:
	./scripts/bundle-android-packs.sh

android: android-native android-packs
	cd android && ./gradlew :app:assembleDebug

beta:
	./scripts/build-beta-apk.sh

clean:
	cargo clean --manifest-path $(CARGO_MANIFEST)
	rm -rf android/app/build android/build
