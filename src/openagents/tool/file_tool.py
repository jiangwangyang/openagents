import anyio


async def execute(cmd_and_args: list[str], work_dir: str) -> tuple[str, bool]:
    # 1. file read <file_path>
    if len(cmd_and_args) == 3 and cmd_and_args[0] == "file" and cmd_and_args[1] == "read":
        file_path = cmd_and_args[2]
        path = await (anyio.Path(work_dir) / file_path).resolve()
        if not await path.exists():
            return f"File not found: {file_path}", True
        if not await path.is_file():
            return f"Path is not file: {file_path}", True
        content = await path.read_text(encoding="utf-8")
        return content, False
    # 2. file write <file_path> <content>
    elif len(cmd_and_args) == 4 and cmd_and_args[0] == "file" and cmd_and_args[1] == "write":
        file_path, content = cmd_and_args[2], cmd_and_args[3]
        path = await (anyio.Path(work_dir) / file_path).resolve()
        await path.parent.mkdir(parents=True, exist_ok=True)
        await path.write_text(content, encoding="utf-8")
        return "", False
    # 3. file edit <file_path> <old_str> <new_str>
    elif len(cmd_and_args) == 5 and cmd_and_args[0] == "file" and cmd_and_args[1] == "edit":
        file_path, old_str, new_str = cmd_and_args[2], cmd_and_args[3], cmd_and_args[4]
        # 读文件
        path = await (anyio.Path(work_dir) / file_path).resolve()
        if not await path.exists():
            return f"File not found: {file_path}", True
        if not await path.is_file():
            return f"Path is not file: {file_path}", True
        content = await path.read_text(encoding="utf-8")
        # 应用替换逻辑
        if old_str not in content:
            return f"Target string not found in file:\n{old_str}", True
        content = content.replace(old_str, new_str)
        # 写文件
        await path.write_text(content, encoding="utf-8")
        return "", False
    return "Unknown command", True
