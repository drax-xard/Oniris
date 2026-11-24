CARGO_MANIFEST := core/Cargo.toml
CLI := cargo run --manifest-path $(CARGO_MANIFEST) -p oniris-cli --
PYTHON ?= python3

.PHONY: all test fmt fmt-check clippy packs packs-wav noise procedural pack qa render android-native android-packs android beta clean spec audition release

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

# Regenerate the procedural packs as WAV (same synthesis as the Opus build) and
# run them through the audio QA gate: loudness, true peak, band balance, loop
# seams.
qa: packs-wav
	$(PYTHON) tools/audio-qa/measure.py --check packs-wav/*/audio/*.wav

render:
	$(CLI) render --pack packs/test-rain --pack packs/test-fire --minutes 10 --out oniris-render.wav

# Build one recorded pack from its JSON spec: make spec SPEC=seaside
# Sources live outside the repo; output lands in dist/ (ignored by git).
spec:
	$(PYTHON) tools/asset-pipeline/ingest.py build tools/asset-pipeline/specs/$(SPEC).json

# Render a 6 minute mix of the built pack for listening
audition:
	$(CLI) render --pack dist/packs/$(SPEC) --minutes 6 --out dist/work/$(SPEC)-audition.wav

# Build one recorded pack and publish it as a GitHub release asset
release:
	PYTHON=$(PYTHON) ./scripts/release-pack.sh $(SPEC)

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
