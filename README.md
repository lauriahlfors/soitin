# Soitin

A local, cross-platform desktop music player.

- No accounts
- Local only, bring your own media
- Opinionated UI

# Stack 

List of technical development tools and libraries used in this project and reasons for their use.

Framework
- **Tauri** - App framework for cross-plarform developing

Frontend
- **Svelte** - UI rendering in the Tauri webview
- **TypeScript**
- **TailwindCSS**
- **Bits UI**

Backend
- **Rust** - Filesystem access, persistent media database
- **Serde** - Music file metadata access

Devops
- **Git** - Version control

_**Dev note**: I really didn't feel the need to use Docker on this project as Tauri apps are native GUI binaries and Docker cannot be used to run them for the user._

# Feature Roadmap

- Backend
	- [ ] Filesystem access (media library scanning)
	- [ ] Persistent music media database (tracks, metadata, playlists)
	- [ ] Data streaming to frontend

- Frontend
	- [ ] Playback via the native HTML <audio> element
	- [ ] Refined UI & UX

# Acknowledgements
AI such as Claude has been used to accelerate feature development on this project. All of the architectural and visual decisions have been made by humans for humans.

This project is under development and is being developed on MacOS, Windows and Arch Linux devices.
