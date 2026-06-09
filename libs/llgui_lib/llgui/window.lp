class Window {
    private int handle;

    public Window(String title, int width, int height) {
        this.handle = cgui.create_window(title, width, height);
    }

    public void setTitle(String title) {
        cgui.set_window_title(this.handle, title);
    }

    public void clear(int r, int g, int b) {
        cgui.clear_window(this.handle, r, g, b);
    }

    public void show() {
        cgui.show_window(this.handle);
    }

    public void run() {
        while (!this.shouldClose()) {
            this.pollEvents();
            this.swapBuffers();
            this.sleep(1);
        }
        this.destroy();
    }

    public int shouldClose() {
        return cgui.window_should_close(this.handle);
    }

    public void pollEvents() {
        cgui.poll_events(this.handle);
    }

    public void swapBuffers() {
        cgui.swap_buffers(this.handle);
    }

    public void sleep(int ms) {
        cgui.sleep_ms(ms);
    }

    public void destroy() {
        cgui.destroy_window(this.handle);
    }
}