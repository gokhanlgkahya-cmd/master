set -e
export ANDROID_HOME=/opt/android-sdk
mkdir -p $ANDROID_HOME/cmdline-tools && cd /tmp
echo "== cmdline-tools"
curl -sSfL -o cmdtools.zip https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip
rm -rf $ANDROID_HOME/cmdline-tools/latest && unzip -q cmdtools.zip -d $ANDROID_HOME/cmdline-tools && mv $ANDROID_HOME/cmdline-tools/cmdline-tools $ANDROID_HOME/cmdline-tools/latest && rm cmdtools.zip
SDKM=$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager
echo "== lisanslar"; yes | $SDKM --licenses >/dev/null 2>&1 || true
echo "== paketler"
$SDKM --install "platform-tools" "platforms;android-34" "build-tools;34.0.0" "ndk;27.2.12479018" 2>&1 | tail -3
echo "== rust hedefleri"
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
echo "== tauri-cli"
cargo install tauri-cli --version "^2" --locked 2>&1 | tail -2
echo ANDROID_SETUP_DONE
