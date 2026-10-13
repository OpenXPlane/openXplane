#!/usr/bin/env python3
"""Trace the two native menu-command branches with explicit query returns and logged UI helpers."""
import sys
from emulate_xp import Emulator
from extract_commands import REFERENCE_SHA256, detailed_records
from unicorn.x86_const import UC_X86_REG_RAX, UC_X86_REG_RCX, UC_X86_REG_RDX, UC_X86_REG_R8


def main(path):
    records = detailed_records(path)
    assert records[29][0] == 'sim/operation/toggle_main_menu'
    assert records[30][0] == 'sim/operation/toggle_settings_window'
    emu = Emulator(path)
    owner = emu.alloc(0x2b0)
    log = []
    supplied = [False, False]

    def window_query(e):
        assert e.reg(UC_X86_REG_RCX) == owner and e.reg(UC_X86_REG_RDX) == 11
        log.append('QueryMainMenu')
        e.uc.reg_write(UC_X86_REG_RAX, int(supplied[0]))

    def state_query(e):
        assert e.reg(UC_X86_REG_RCX) == 0x142f2d770
        log.append('QueryState')
        e.uc.reg_write(UC_X86_REG_RAX, int(supplied[1]))

    def effect(name, kind=None, flag=None):
        def call(e):
            assert e.reg(UC_X86_REG_RCX) == owner
            if kind is not None:
                assert e.reg(UC_X86_REG_RDX) == kind
            if flag is not None:
                assert e.reg(UC_X86_REG_R8) & 255 == flag
            log.append(name)
        return call

    emu.stubs.update({
        0x141512eb0: window_query,
        0x1417fb320: state_query,
        0x14150ce20: effect('CloseMainMenu', 11, 1),
        0x14150e900: effect('EnsureMainMenu', 11, 0),
        0x14150d370: effect('Refresh'),
        0x14150fe90: effect('ToggleSettingsWindow', 12),
    })
    print('# reference_sha256', REFERENCE_SHA256)
    print('# local native handler 0x14150bfe0; all six helper bodies stubbed; conditional routing only')
    print('# M command window_query state_query -> ordered helper requests')
    for command in [29, 30]:
        for window in [False, True]:
            for state in [False, True]:
                supplied[:] = [window, state]
                log.clear()
                emu.call(0x14150bfe0, ints=[owner, command], max_instructions=1000)
                print('M', command, int(window), int(state), '->', *log)


if __name__ == '__main__':
    main(sys.argv[1])
