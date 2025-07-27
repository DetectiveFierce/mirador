@echo off
REM Ensure a commit message is provided
if "%~1"=="" (
    echo Usage: push "commit message"
    exit /b 1
)

REM Store the commit message
set "message=%~1"

REM Clean and regenerate docs
cargo clean --doc
cargo doc --no-deps --release

REM Deploy docs to GitHub Pages
ghp-import -n -p -f target/doc

REM Remove all but the most recent maze file in the generated folder, if any exist
set "generated_dir=.\src\game\maze\saved-mazes\generated"
if exist "%generated_dir%" (
    pushd "%generated_dir%" || exit /b 1
    
    REM Get list of files sorted by modification time (newest first)
    for /f "delims=" %%i in ('dir /b /o-d /a-d 2^>nul') do (
        if not defined newest_file (
            set "newest_file=%%i"
        ) else (
            echo Deleting: %%i
            del "%%i"
        )
    )
    
    popd
)

REM Stage, commit, and push code changes
git add .
git commit -m "%message%"
git push -u origin main