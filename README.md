# Text Fixer
> A general-purpose project with the goal of creating a good interface for writing with LLMs.

## Installation
Make sure you have ```npm``` and ```rust``` installed. Clone the repository and run:
```bash
npm run tauri dev
```

## TODO
- [ ] Copy text to "fix" it.
- [ ] Add *tab* to see other recommendations.
- [ ] Window should hover over input text
  - [ ] Temp. fix: center text-fixer on current application.
    - [x] fix: displaying the window center is currently offset.
  - [ ] A better implementation would be to match the text-fixer window size to the input window size. This way you could also add a border around the current window making it very clear which window is being used.
  - [ ] This would also make the centering of the input field a lot easier. 
- [ ] Consider adding github "before/after" change view.
- [ ] Add information about what app is in focus. 
  - [ ] Can you get the icon? <- this is difficult

- [ ] Input text styling
  - [x] Add icon
  - [x] Add animation when text is sent
  - [ ] Add popup animation
