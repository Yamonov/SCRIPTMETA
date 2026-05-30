#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PROJECT="$ROOT/scriptmetakitApp/scriptmetakitApp.xcodeproj"
SCHEME="scriptmetakitApp"
APP_NAME="scriptmetakitApp"
DERIVED_DATA="$ROOT/build/DerivedData"

pkill -x "$APP_NAME" 2>/dev/null || true

xcodebuild \
  -project "$PROJECT" \
  -scheme "$SCHEME" \
  -configuration Debug \
  -derivedDataPath "$DERIVED_DATA" \
  CODE_SIGNING_ALLOWED=NO \
  build

APP_PATH="$DERIVED_DATA/Build/Products/Debug/$APP_NAME.app"
if [[ ! -d "$APP_PATH" ]]; then
  echo "App bundle was not found: $APP_PATH" >&2
  exit 1
fi

open -n "$APP_PATH"
