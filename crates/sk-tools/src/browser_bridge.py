import sys
import json
import asyncio
import os
from playwright.async_api import async_playwright

class SovereignBrowser:
    def __init__(self):
        self.playwright = None
        self.browser = None
        self.context = None
        self.page = None

    async def start(self):
        self.playwright = await async_playwright().start()
        self.browser = await self.playwright.chromium.launch(headless=True)
        
        # Session Vault: Load existing state if available
        storage_state = "browser_session.json" if os.path.exists("browser_session.json") else None
        
        self.context = await self.browser.new_context(
            user_agent="Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            storage_state=storage_state
        )
        # Stealth headers
        await self.context.set_extra_http_headers({"Accept-Language": "en-US,en;q=0.9"})
        self.page = await self.context.new_page()
        await self.page.set_viewport_size({"width": 1920, "height": 1080})

    async def stop(self):
        # Save session before closing
        if self.context:
            await self.context.storage_state(path="browser_session.json")
        if self.browser:
            await self.browser.close()
        if self.playwright:
            await self.playwright.stop()

    async def execute(self, command, args):
        try:
            if command == "navigate":
                url = args.get("url")
                await self.page.goto(url, wait_until="domcontentloaded", timeout=30000)
                # Handle common redirects
                content = await self.page.content()
                if "non-JavaScript site" in content or "Access Denied" in content:
                    await self.page.wait_for_timeout(2000)
                    links = await self.page.query_selector_all("a")
                    if links: await links[0].click()
                    await self.page.wait_for_load_state("networkidle")
                return {"status": "success", "url": self.page.url}

            elif command == "read_page":
                url = args.get("url")
                if url:
                    await self.page.goto(url, wait_until="domcontentloaded", timeout=30000)
                
                # Wait for potential dynamic content
                await self.page.wait_for_timeout(1000)
                text = await self.page.evaluate("() => document.body.innerText")
                if len(text) < 300:
                    text = await self.page.content()
                return {"status": "success", "text": text}

            elif command == "click":
                selector = args.get("selector")
                await self.page.click(selector, timeout=5000)
                return {"status": "success", "message": f"Clicked {selector}"}

            elif command == "type":
                selector = args.get("selector")
                text = args.get("text")
                await self.page.fill(selector, text, timeout=5000)
                return {"status": "success", "message": f"Typed text into {selector}"}

            elif command == "scroll":
                direction = args.get("direction", "down")
                if direction == "down":
                    await self.page.evaluate("window.scrollBy(0, 500)")
                else:
                    await self.page.evaluate("window.scrollBy(0, -500)")
                return {"status": "success", "message": f"Scrolled {direction}"}

            elif command == "screenshot":
                path = args.get("path", "screenshot.png")
                await self.page.screenshot(path=path, full_page=True)
                return {"status": "success", "path": path}

            elif command == "get_dom":
                # Return a simplified accessibility-tree like structure
                dom = await self.page.evaluate("""() => {
                    const walk = (node) => {
                        if (node.nodeType === 3) return null;
                        const obj = {
                            tag: node.tagName,
                            id: node.id || undefined,
                            class: node.className || undefined,
                            text: node.innerText ? node.innerText.substring(0, 50).trim() : undefined,
                            children: []
                        };
                        for (let child of node.children) {
                            const c = walk(child);
                            if (c) obj.children.push(c);
                        }
                        return obj;
                    };
                    return walk(document.body);
                }""")
                return {"status": "success", "dom": dom}

            else:
                return {"status": "error", "message": f"Unknown command: {command}"}

        except Exception as e:
            return {"status": "error", "message": str(e)}

async def main():
    browser = SovereignBrowser()
    await browser.start()

    # If run as a one-shot (CLI legacy support)
    if len(sys.argv) > 2:
        cmd = sys.argv[1]
        args = json.loads(sys.argv[2])
        result = await browser.execute(cmd, args)
        print(json.dumps(result))
        await browser.stop()
        return

    # Daemon mode: listen to stdin
    while True:
        line = await asyncio.get_event_loop().run_in_executor(None, sys.stdin.readline)
        if not line:
            break
        try:
            req = json.loads(line)
            cmd = req.get("command")
            args = req.get("args", {})
            result = await browser.execute(cmd, args)
            print(json.dumps(result))
            sys.stdout.flush()
        except Exception as e:
            print(json.dumps({"status": "error", "message": str(e)}))
            sys.stdout.flush()

    await browser.stop()

if __name__ == "__main__":
    asyncio.run(main())
