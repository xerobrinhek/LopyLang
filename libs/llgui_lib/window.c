#include <X11/Xlib.h>
#include <X11/Xutil.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <unistd.h>

static Display* display = NULL;
static Atom wmDeleteMessage;
static int should_close_flag = 0;

__int128 create_window(const char* title, __int128 width, __int128 height) {
    if (display == NULL) {
        display = XOpenDisplay(NULL);
        if (display == NULL) return -1;
        wmDeleteMessage = XInternAtom(display, "WM_DELETE_WINDOW", False);
    }

    int screen = DefaultScreen(display);

    Window window = XCreateSimpleWindow(display, RootWindow(display, screen),
                                        0, 0,
                                        (int)width, (int)height,
                                        0,
                                        BlackPixel(display, screen),
                                        WhitePixel(display, screen));

    if (window == 0) {
        return -1;
    }

    XStoreName(display, window, title);
    XSetWMProtocols(display, window, &wmDeleteMessage, 1);
    XSelectInput(display, window,
        ExposureMask | KeyPressMask | KeyReleaseMask |
        ButtonPressMask | StructureNotifyMask);

    XMapWindow(display, window);
    XFlush(display);

    return (__int128)window;
}

void sleep_ms(long long ms) {
    usleep(ms * 1000);
}

void set_window_title(__int128 handle, const char* title) {
    if (display != NULL && handle != -1) {
        XStoreName(display, (Window)(long long)handle, title);
        XFlush(display);
    }
}

void clear_window(__int128 handle, __int128 r, __int128 g, __int128 b) {
    if (display != NULL && handle != -1) {
        unsigned long color = (((long long)r & 0xFF) << 16) | (((long long)g & 0xFF) << 8) | ((long long)b & 0xFF);
        XSetWindowBackground(display, (Window)(long long)handle, color);
        XClearWindow(display, (Window)(long long)handle);
        XFlush(display);
    }
}

void show_window(__int128 handle) {
    if (display != NULL && handle != -1) {
        XMapWindow(display, (Window)(long long)handle);
        XFlush(display);
    }
}

void poll_events(__int128 handle) {
    if (display == NULL || handle == -1) return;

    while (XPending(display) > 0) {
        XEvent event;
        XNextEvent(display, &event);

        switch (event.type) {
            case ClientMessage:
                if ((Atom)event.xclient.data.l[0] == wmDeleteMessage) {
                    should_close_flag = 1;
                }
                break;
            case KeyPress:
                {
                    char key[32];
                    KeySym keysym;
                    XLookupString(&event.xkey, key, sizeof(key), &keysym, NULL);
                    if (keysym == XK_Escape || keysym == XK_q) {
                        should_close_flag = 1;
                    }
                }
                break;
            case DestroyNotify:
                should_close_flag = 1;
                break;
            default:
                break;
        }
    }
}

__int128 window_should_close(__int128 handle) {
    return should_close_flag;
}

void swap_buffers(__int128 handle) {
    if (display != NULL) {
        XFlush(display);
    }
}

void destroy_window(__int128 handle) {
    if (display != NULL && handle != -1) {
        XDestroyWindow(display, (Window)(long long)handle);
    }
}