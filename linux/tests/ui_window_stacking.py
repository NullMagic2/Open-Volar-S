#!/usr/bin/env python3
"""Verify independent LiveTV stacking with actual XFWM activation.

Usage: python3 linux/tests/ui_window_stacking.py /path/to/open-volar-s-live-tv
Requires Xvfb, xfwm4, xdotool, xmessage, and dbus-run-session.
Uses an isolated display/configuration and never opens the tuner.
"""
import ctypes as c
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import time


def run(*args):
    return subprocess.check_output(args, text=True, timeout=8).strip()


def send_message(window, message, values):
    # Target the viewer directly; in paired mode the WM may focus the DAC, so a
    # global Alt+F10 shortcut could act on the controls instead of the viewer.
    class Data(c.Union):
        _fields_ = [('l', c.c_long * 5)]
    class Message(c.Structure):
        _fields_ = [('type', c.c_int), ('serial', c.c_ulong), ('send_event', c.c_int),
                    ('display', c.c_void_p), ('window', c.c_ulong), ('message_type', c.c_ulong),
                    ('format', c.c_int), ('data', Data)]
    class Event(c.Union):
        _fields_ = [('message', Message), ('pad', c.c_long * 24)]
    x = c.CDLL('libX11.so.6')
    x.XOpenDisplay.argtypes = [c.c_char_p]; x.XOpenDisplay.restype = c.c_void_p
    x.XDefaultRootWindow.argtypes = [c.c_void_p]; x.XDefaultRootWindow.restype = c.c_ulong
    x.XInternAtom.argtypes = [c.c_void_p, c.c_char_p, c.c_int]; x.XInternAtom.restype = c.c_ulong
    x.XSendEvent.argtypes = [c.c_void_p, c.c_ulong, c.c_int, c.c_long, c.POINTER(Event)]
    x.XSync.argtypes = [c.c_void_p, c.c_int]; x.XCloseDisplay.argtypes = [c.c_void_p]
    display = x.XOpenDisplay(None)
    assert display, 'Cannot open test display'
    atom = lambda name: x.XInternAtom(display, name.encode(), 0)
    event = Event()
    event.message = Message(33, 0, 1, display, int(window), atom(message), 32,
                            Data((c.c_long * 5)(*[atom(v) if isinstance(v,str) else v for v in values])))
    x.XSendEvent(display, x.XDefaultRootWindow(display), 0, (1 << 19) | (1 << 20), c.byref(event))
    x.XSync(display, 0); x.XCloseDisplay(display)


def maximize(window, enabled):
    send_message(window,'_NET_WM_STATE',[int(enabled),'_NET_WM_STATE_MAXIMIZED_VERT',
                                       '_NET_WM_STATE_MAXIMIZED_HORZ',1,0])


def restack_below(window, sibling):
    send_message(window,'_NET_RESTACK_WINDOW',[2,int(sibling),1,0,0])


def session(binary, root):
    processes = []
    log = open(Path(root) / 'desktop.log', 'w')
    read_fd, write_fd = os.pipe()
    try:
        server = subprocess.Popen(
            ['Xvfb', '-displayfd', str(write_fd), '-screen', '0', '1920x1080x24',
             '-ac', '-nolisten', 'tcp'], pass_fds=(write_fd,), stdout=log, stderr=log)
        processes.append(server)
        os.close(write_fd)
        with os.fdopen(read_fd) as ready:
            display = ready.readline().strip()
        assert display.isdigit(), 'Xvfb did not start'
        os.environ['DISPLAY'] = ':' + display
        processes.append(subprocess.Popen(['xfwm4'], stdout=log, stderr=log))
        for _ in range(50):
            if 'window id #' in run('xprop', '-root', '_NET_SUPPORTING_WM_CHECK'):
                break
            time.sleep(.1)
        else:
            raise AssertionError('XFWM did not start')
        # Match the user's XFWM activation configuration, including custom
        # borderless client areas (the fresh-profile default differs here).
        for key in ('click_to_focus', 'raise_on_click', 'raise_with_any_button'):
            run('xfconf-query', '-c', 'xfwm4', '-p', '/general/'+key, '-n', '-t', 'bool', '-s', 'true')
        panel_config=Path(os.environ['XDG_CONFIG_HOME'])/'xfce4/xfconf/xfce-perchannel-xml/xfce4-panel.xml'
        panel_config.parent.mkdir(parents=True,exist_ok=True)
        panel_config.write_text('<?xml version="1.0" encoding="UTF-8"?>\n<channel name="xfce4-panel" version="1.0"><property name="configver" type="int" value="2"/>\n<property name="panels" type="array"><value type="int" value="1"/>\n<property name="panel-1" type="empty"><property name="position" type="string" value="p=6;x=0;y=0"/>\n<property name="length" type="double" value="100"/><property name="size" type="uint" value="48"/>\n<property name="position-locked" type="bool" value="true"/>\n<property name="plugin-ids" type="array"><value type="int" value="1"/></property></property></property>\n<property name="plugins" type="empty"><property name="plugin-1" type="string" value="tasklist">\n<property name="grouping" type="uint" value="0"/><property name="show-labels" type="bool" value="true"/>\n</property></property></channel>')
        processes.append(subprocess.Popen(['xfce4-panel'],stdout=log,stderr=log))
        time.sleep(.5)
        app = subprocess.Popen([binary], stdout=log, stderr=log)
        processes.append(app)
        windows = {}
        for _ in range(80):
            found = subprocess.run(['xdotool', 'search', '--onlyvisible', '--pid', str(app.pid)],
                                   capture_output=True, text=True).stdout.split()
            for window in found:
                name = run('xdotool', 'getwindowname', window)
                if name == 'Live TV! — Open Volar S':
                    windows['viewer'] = window
                elif name == 'Live TV! · Receiver':
                    windows['dac'] = window
            if len(windows) == 2:
                break
            assert app.poll() is None, 'LiveTV exited during startup'
            time.sleep(.1)
        assert len(windows) == 2, 'LiveTV windows did not map'
        time.sleep(1.5)  # Allow the bounded startup placement timers to settle.
        viewer, dac = windows['viewer'], windows['dac']
        run('xdotool', 'windowmove', viewer, '600', '100')
        run('xdotool', 'windowmove', dac, '650', '650')
        probe = subprocess.Popen(['xmessage', '-name', 'ovs-stacking-probe', '-title',
                                  'OVS stacking probe', '-geometry', '300x120+30+300', 'Probe'],
                                 stdout=log, stderr=log)
        processes.append(probe)
        probe_window = run('xdotool', 'search', '--sync', '--onlyvisible', '--name', '^OVS stacking probe$').split()[0]
        time.sleep(.3)
        geometry = {w: run('xdotool', 'getwindowgeometry', '--shell', w) for w in (viewer, dac)}
        def restore_pair(first):
            for w in (viewer, dac):
                run('xdotool', 'windowminimize', w)
            time.sleep(.3)
            assert all('_NET_WM_STATE_HIDDEN' in run('xprop', '-id', w, '_NET_WM_STATE')
                       for w in (viewer, dac)), 'Both test windows must be minimized'
            run('xdotool', 'windowactivate', first)
            for _ in range(30):
                if all('_NET_WM_STATE_HIDDEN' not in run('xprop', '-id', w, '_NET_WM_STATE')
                       for w in (viewer, dac)):
                    break
                time.sleep(.1)
            else:
                raise AssertionError('Taskbar restoration left one LiveTV window minimized')
            time.sleep(.3)
        def stack():
            return [int(n,16) for n in re.findall(r'0x[0-9a-f]+',
                    run('xprop','-root','_NET_CLIENT_LIST_STACKING'))]
        def assert_pair_forward(active):
            ordering=stack()
            assert all(ordering.index(int(w))>ordering.index(int(probe_window))
                       for w in (viewer,dac)), f'One window stayed behind another application: {ordering}'
            assert int(run('xdotool','getactivewindow'))==int(active), f'Activation changed keyboard focus: wanted {active}, got {run("xdotool","getactivewindow")}'
            time.sleep(.3)
            assert stack()==ordering, 'Windows kept restacking after activation'
        def external_activations():
            for cycle in range(8):
                active,peer=(viewer,dac) if cycle%2==0 else (dac,viewer)
                for w in (peer,active,probe_window):
                    run('xdotool','windowraise',w)
                run('xdotool','windowactivate','--sync',probe_window)
                time.sleep(.2)
                run('xdotool','windowactivate',active)
                time.sleep(.3)
                assert_pair_forward(active)
            print('PASS: entering LiveTV from another app brings both covered windows forward, without a focus loop.')
        def internal_activations():
            for cycle in range(8):
                clicked,peer=(dac,viewer) if cycle%2==0 else (viewer,dac)
                if cycle%2:
                    run('xdotool','mousemove','--window',clicked,'500','150','click','1')
                else:
                    run('xdotool','windowactivate',clicked)
                time.sleep(.3)
                ordering=stack()
                assert ordering.index(int(clicked))>ordering.index(int(peer)), (
                    f'Clicked LiveTV member did not stay in front: {ordering}')
                assert int(run('xdotool','getactivewindow'))==int(clicked), 'Internal click changed keyboard focus'
                time.sleep(.3)
                assert stack()==ordering, 'Internal activation kept changing stacking'
                for w in (viewer,dac):
                    assert 'ABOVE' not in run('xprop','-id',w,'_NET_WM_STATE'), 'Window acquired always-on-top state'
                    assert 'window id #' not in run('xprop','-id',w,'WM_TRANSIENT_FOR'), 'Window acquired transient-parent state'
            print('PASS: clicking between viewer and DAC preserves the clicked window in front without repeated restacking or parent/topmost hints.')
        import gi
        gi.require_version('Atspi','2.0')
        from gi.repository import Atspi
        def accessibles(node):
            yield node
            for i in range(node.get_child_count()):
                child=node.get_child_at_index(i)
                if child is not None:
                    yield from accessibles(child)
        buttons={}
        for _ in range(30):
            for node in accessibles(Atspi.get_desktop(0)):
                name=node.get_name() or ''
                if name in ('Live TV! — Open Volar S','Live TV! · Receiver') and node.get_role_name()=='toggle button':
                    rect=node.get_component_iface().get_extents(Atspi.CoordType.SCREEN)
                    if rect.width>0:buttons[name]=(rect.x+rect.width//2,rect.y+rect.height//2)
            if len(buttons)==2:break
            time.sleep(.1)
        assert len(buttons)==2, f'Could not locate actual XFCE task buttons: {buttons}'
        print('Taskbar buttons:',buttons,flush=True)
        def click_taskbar(window):
            name='Live TV! — Open Volar S' if window==viewer else 'Live TV! · Receiver'
            for node in accessibles(Atspi.get_desktop(0)):
                if node.get_name()==name and node.get_role_name()=='toggle button':
                    rect=node.get_component_iface().get_extents(Atspi.CoordType.SCREEN)
                    buttons[name]=(rect.x+rect.width//2,rect.y+rect.height//2)
            x,y=buttons[name]
            print('Click taskbar',name,x,y,flush=True)
            run('xdotool','mousemove',str(x),str(y),'click','1')
            time.sleep(.4)
        for first in (viewer,dac):
            run('xdotool','windowactivate','--sync',probe_window)
            time.sleep(.2)
            click_taskbar(first)
            assert_pair_forward(first)
            for w in (viewer,dac):run('xdotool','windowminimize',w)
            time.sleep(.3)
            click_taskbar(first)
            assert all('HIDDEN' not in run('xprop','-id',w,'_NET_WM_STATE') for w in (viewer,dac)), 'Real taskbar left a minimized peer'
            assert_pair_forward(first)
        print('PASS: clicking both actual XFCE task buttons restores both minimized or covered windows.')
        external_activations()
        internal_activations()
        for w in (viewer,dac):
            assert run('xdotool','getwindowgeometry','--shell',w)==geometry[w], 'Activation moved a window'
        for first in (viewer,dac):
            run('xdotool','windowactivate','--sync',probe_window)
            restore_pair(first)
            assert_pair_forward(first)
        print('PASS: restoring either minimized window brings both above the other application.')
        maximize(viewer,True)
        time.sleep(.5)
        assert 'MAXIMIZED_VERT' in run('xprop','-id',viewer,'_NET_WM_STATE')
        external_activations()
        internal_activations()
        for first in (viewer,dac):
            run('xdotool','windowactivate','--sync',probe_window)
            restore_pair(first)
            assert_pair_forward(first)
        print('PASS: the same activation/restoration behavior works with a maximized viewer.')
        maximize(viewer,False)
        time.sleep(.5)
        assert 'MAXIMIZED' not in run('xprop','-id',viewer,'_NET_WM_STATE')
        external_activations()
        internal_activations()
    finally:
        for process in reversed(processes):
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
        log.close()
        if sys.exc_info()[0]:
            print((Path(root) / 'desktop.log').read_text(), file=sys.stderr)


if __name__ == '__main__':
    if len(sys.argv) == 4 and sys.argv[1] == '--session':
        session(sys.argv[2], sys.argv[3])
    else:
        binary = str(Path(sys.argv[1]).resolve())
        with tempfile.TemporaryDirectory(prefix='ovs-stacking-', ignore_cleanup_errors=True) as root:
            env = os.environ.copy()
            for key in ('WAYLAND_DISPLAY', 'GDK_SCALE', 'GDK_DPI_SCALE', 'WSL_DISTRO_NAME', 'SESSION_MANAGER'):
                env.pop(key, None)
            env.update(GDK_BACKEND='x11', XDG_CURRENT_DESKTOP='XFCE', XDG_SESSION_DESKTOP='xfce', DESKTOP_SESSION='xfce', XDG_CONFIG_HOME=root+'/config',
                       XDG_CACHE_HOME=root+'/cache', XDG_DATA_HOME=root+'/data')
            result = subprocess.run(['dbus-run-session', '--', sys.executable,
                                     str(Path(__file__).resolve()), '--session', binary, root], env=env)
            sys.exit(result.returncode)
