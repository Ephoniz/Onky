use audio_program_backend::VirtualCable;

use super::cable_strips;

#[test]
fn cables_become_virtual_strips() {
    let cables = [
        VirtualCable {
            node_name: "audio-program-virtual-input-1".to_string(),
            description: "Audio Program Virtual Input 1".to_string(),
            index: 0,
            output: false,
        },
        VirtualCable {
            node_name: "audio-program-virtual-output-1".to_string(),
            description: "Audio Program Virtual Output 1".to_string(),
            index: 0,
            output: true,
        },
    ];
    let strips = cable_strips(&cables);
    assert_eq!(strips[0].name, "Audio Program Virtual Input 1");
    assert_eq!(strips[0].kind, "Virtual input");
    assert!(!strips[0].output);
    assert_eq!(strips[1].kind, "Virtual output");
    assert!(strips[1].output);
    assert!(strips
        .iter()
        .all(|strip| strip.status.is_none() && strip.master.is_none()));
}
