# axiom-profiler-yew-GUI
## How to run:
- Clone repository.
- Ensure Actix-Web server for this project has been started (see https://github.com/richardluo20/axiom-profiler-rust-prototype).
- In the top-level directory of the crate, enter `trunk serve` in terminal and then open http://127.0.0.1:8080 in a browser,
or `trunk serve --open` to open it in browser once all compilation/building has completed.
It may still be necessary to switch windows manually. Server must be stopped manually via the terminal (i.e. Ctrl+C or Cmd+C).
- With the app loaded in the browser, clicking "Browse..." will open up a file upload dialog box.
Select a trace file you would like to use the parser on and check that the displayed file name is that of the correct file.
- A button labelled "Load file" will appear. Click it to send the file to the backend. When the parser has finished, the resulting SVG graph will be sent back and appear on the page.
Depending on the size of the submitted trace file, this processing may take a considerable amount of time.
