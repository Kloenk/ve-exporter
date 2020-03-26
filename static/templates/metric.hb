# HELP ve_up 1 if the connection is up
# TYPE ve_up gauge
ve_up{model="{{pid}}",serial="{{serialNumber}}"} {{#if online}} 1 {{^}} 0 {{/if}}
ve_op_state{model="{{pid}}",serial="{{serialNumber}}"} {{state}}
ve_battery_current{model="{{pid}}",serial="{{serialNumber}}"} {{current}}
ve_yield_total_user{model="{{pid}}",serial="{{serialNumber}}"} {{yieldTotalUser}}
ve_hsds{model="{{pid}}",serial="{{serialNumber}}"} {{day}}
ve_yield_total{model="{{pid}}",serial="{{serialNumber}}"} {{yieldTotal}}
ve_power_max{model="{{pid}}",serial="{{serialNumber}}",day="yesterday"} {{maxPowerYesterday}}
ve_load{model="{{pid}}",serial="{{serialNumber}}"} {{#if load}} 1 {{^}} 0 {{/if}}
ve_power_panel{model="{{pid}}",serial="{{serialNumber}}"} {{pannelPower}}
ve_current_load{model="{{pid}}",serial="{{serialNumber}}"} {{loadCurrent}}
ve_voltage_panel{model="{{pid}}",serial="{{serialNumber}}"} {{panelVoltage}}
ve_yield{model="{{pid}}",serial="{{serialNumber}}",day="yesterday"} {{yieldYesterday}}
ve_firmware{model="{{pid}}",serial="{{serialNumber}}",bit="16"} {{firmware16}}
ve_power_max{model="{{pid}}",serial="{{serialNumber}}",day="today"} {{maxPowerToday}}
ve_voltage{model="{{pid}}",serial="{{serialNumber}}"} {{voltageCurrent}}
