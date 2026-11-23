cask "nova-studio" do
  version "0.1.0"
  sha256 "6c0deeabb5b719b2ca51dfad46508959a3b585ce75b50f47729f30fb9b4cb589"

  url "https://github.com/jd4rider/nova/releases/download/v#{version}/Nova-Studio-#{version}-arm64.dmg"
  name "Nova Studio"
  desc "Desktop IDE for the Nova language"
  homepage "https://github.com/jd4rider/nova"

  depends_on arch: :arm64
  depends_on macos: :big_sur
  depends_on formula: "jd4rider/tap/nova"

  app "Nova Studio.app"

  caveats <<~EOS
    The Nova language (`nova` CLI) is installed automatically from jd4rider/tap.
    Nova Studio is unsigned and not notarized. If macOS blocks the first launch,
    right-click the app and choose Open, or run:
      xattr -dr com.apple.quarantine "/Applications/Nova Studio.app"
  EOS

  zap trash: [
    "~/Library/Application Support/Nova Studio",
    "~/Library/Preferences/dev.nova.studio.plist",
    "~/Library/Saved Application State/dev.nova.studio.savedState",
  ]
end
