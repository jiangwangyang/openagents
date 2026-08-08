import uvicorn

from openagents import application

_config = uvicorn.Config(application.app, host="127.0.0.1", port=8000, log_level="INFO", workers=1, access_log=False)
_server = uvicorn.Server(_config)


def main() -> None:
    try:
        _server.run()
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()
