CARGO_MANIFEST := core/Cargo.toml
CLI := cargo run --manifest-path $(CARGO_MANIFEST) -p oniris-cli --
PYTHON ?= python3

.PHONY: all test fmt fmt-check clippy packs qa render android-native android clean

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

# Regenerate the procedural packs, then run them through the audio QA gate.
qa: packs
	$(PYTHON) tools/audio-qa/measure.py --check packs/*/audio/*.wav

render:
	$(CLI) render --pack packs/test-rain --pack packs/test-fire --minutes 10 --out oniris-render.wav

android-native:
	cd core && cargo ndk \
		-t arm64-v8a -t armeabi-v7a -t x86_64 \
		-o ../android/app/src/main/jniLibs \
		build -p oniris-jni --release

android: android-native
	cd android && ./gradlew :app:assembleDebug

clean:
	cargo clean --manifest-path $(CARGO_MANIFEST)
	rm -rf android/app/build android/build
