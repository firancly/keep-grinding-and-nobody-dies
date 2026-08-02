#!/usr/bin/env python3

"""Apply the handmade pixel-art theme to the current repository.



Run from the repository root after copying this patch into it:

    python apply_handmade_theme.py



The script only changes the tablet/player presentation. It does not touch

Rust, firmware, serial protocol, configuration, or game rules.

"""



from pathlib import Path

import re

import sys



ROOT = Path(__file__).resolve().parent

TABLET = ROOT / "src-tauri" / "src" / "tablet.html"

THEME = ROOT / "src-tauri" / "src" / "tablet-theme.css"



START = "<!-- HANDMADE_PIXEL_THEME_START -->"

END = "<!-- HANDMADE_PIXEL_THEME_END -->"





def fail(message: str) -> None:

    raise SystemExit(message)





def patch_memory_stage(html: str) -> str:

    function_marker = "function renderMemory(mod) {"

    stage_logic_marker = "const stageDots = Array.from({ length: 5 }"



    stage_setup = '''function renderMemory(mod) {

        const stageDots = Array.from({ length: 5 }, (_, i) => {

          const stageClass =

            i < mod.stage - 1

              ? "is-done"

              : i === mod.stage - 1

                ? "is-current"

                : "";

          return `<span class="memory-stage-dot ${stageClass}"></span>`;

        }).join("");'''



    if stage_logic_marker not in html:

        if function_marker not in html:

            fail(

                "Could not find renderMemory(mod) in tablet.html. "

                "The repository structure may have changed."

            )

        html = html.replace(function_marker, stage_setup, 1)



    old_stage = '<div class="memory-stage">STAGE ${mod.stage} / 5</div>'

    new_stage = (

        '<div class="memory-stage" '

        'aria-label="Memory stage ${mod.stage} of 5">${stageDots}</div>'

    )



    if old_stage in html:

        html = html.replace(old_stage, new_stage, 1)

    elif new_stage not in html:

        fail(

            "Could not find the Memory stage markup in tablet.html. "

            "The repository structure may have changed."

        )



    return html





def main() -> None:

    if not TABLET.exists():

        fail(

            f"Missing {TABLET}. Copy this patch into the repository root "

            "before running it."

        )



    if not THEME.exists():

        fail(f"Missing theme file: {THEME}")



    html = TABLET.read_text(encoding="utf-8")

    theme = THEME.read_text(encoding="utf-8").strip()



    backup = TABLET.with_suffix(TABLET.suffix + ".bak")

    if not backup.exists():

        backup.write_text(html, encoding="utf-8")



    # Remove an earlier injected theme so rerunning stays clean and idempotent.

    html = re.sub(

        re.escape(START) + r".*?" + re.escape(END),

        "",

        html,

        flags=re.DOTALL,

    )



    html = patch_memory_stage(html)



    injection = (

        f"\n{START}\n"

        '<style id="handmade-pixel-theme">\n'

        f"{theme}\n"

        "</style>\n"

        f"{END}\n"

    )



    if "</head>" not in html:

        fail("tablet.html does not contain a </head> tag.")



    html = html.replace("</head>", injection + "</head>", 1)

    TABLET.write_text(html, encoding="utf-8")



    print("Handmade pixel theme applied.")

    print(f"Updated: {TABLET}")

    print(f"Backup:  {backup}")

    print("Expert styling is supplied by src/defuser.css in this patch.")





if __name__ == "__main__":

    main()