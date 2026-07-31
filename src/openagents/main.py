import threading
import webbrowser

import uvicorn

from openagents import application

_config = uvicorn.Config(application.app, host="0.0.0.0", port=8000, log_level="INFO", workers=1, access_log=False)
_server = uvicorn.Server(_config)


def _open_browser():
    application.startup_event.wait()
    webbrowser.open("http://localhost:8000")


def main():
    threading.Thread(target=_open_browser, args=()).start()
    try:
        _server.run()
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()
