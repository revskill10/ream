//! Comprehensive tests for REAM Actor Model Features
//! 
//! Tests the complete actor model implementation including:
//! - Selective receive with pattern matching
//! - Actor linking and monitoring
//! - System message handling
//! - Enhanced actor context functionality

use ream::runtime::actor::*;
use ream::types::{Pid, MessagePayload};
use ream::error::RuntimeResult;
use std::time::Duration;
use std::sync::{Arc, Mutex};

#[test]
fn test_message_pattern_matching() {
    // Test different message pattern types
    let text_msg = MessagePayload::Text("hello".to_string());
    let data_msg = MessagePayload::Data(serde_json::Value::Number(42.into()));
    let bytes_msg = MessagePayload::Bytes(vec![1, 2, 3, 4]);
    
    // Test Any pattern
    let any_pattern = MessagePattern::Any;
    assert!(matches_pattern(&text_msg, &any_pattern));
    assert!(matches_pattern(&data_msg, &any_pattern));
    assert!(matches_pattern(&bytes_msg, &any_pattern));
    
    // Test Text pattern
    let text_pattern = MessagePattern::Text("hello".to_string());
    assert!(matches_pattern(&text_msg, &text_pattern));
    assert!(!matches_pattern(&data_msg, &text_pattern));
    
    let wrong_text_pattern = MessagePattern::Text("world".to_string());
    assert!(!matches_pattern(&text_msg, &wrong_text_pattern));
    
    // Test Type pattern
    let data_type_pattern = MessagePattern::Type(MessageType::Data);
    assert!(matches_pattern(&data_msg, &data_type_pattern));
    assert!(!matches_pattern(&text_msg, &data_type_pattern));
    
    let bytes_type_pattern = MessagePattern::Type(MessageType::Bytes);
    assert!(matches_pattern(&bytes_msg, &bytes_type_pattern));
    assert!(!matches_pattern(&text_msg, &bytes_type_pattern));
    
    // Test Custom pattern
    let custom_pattern = MessagePattern::Custom(|msg| {
        matches!(msg, MessagePayload::Text(text) if text.len() > 3)
    });
    assert!(matches_pattern(&text_msg, &custom_pattern)); // "hello" has length > 3
    assert!(!matches_pattern(&data_msg, &custom_pattern));
}

// Helper function to test pattern matching
fn matches_pattern(message: &MessagePayload, pattern: &MessagePattern) -> bool {
    let context = ActorContext::new(Pid::new());
    context.matches_pattern(message, pattern)
}

#[test]
fn test_selective_receive_with_timeout() {
    let pid = Pid::new();
    let context = ActorContext::new(pid);
    
    // Add messages to mailbox
    {
        let mut mailbox = context.mailbox.lock().unwrap();
        mailbox.push_back(MessagePayload::Text("first".to_string()));
        mailbox.push_back(MessagePayload::Data(serde_json::Value::Number(42.into())));
        mailbox.push_back(MessagePayload::Text("second".to_string()));
        mailbox.push_back(MessagePayload::Bytes(vec![1, 2, 3]));
    }
    
    // Test selective receive for specific text
    let pattern = MessagePattern::Text("second".to_string());
    let result = context.selective_receive(pattern, Some(Duration::from_millis(100))).unwrap();
    assert!(result.is_some());
    if let Some(MessagePayload::Text(text)) = result {
        assert_eq!(text, "second");
    } else {
        panic!("Expected text message");
    }
    
    // Verify message was removed from mailbox
    let mailbox_size = context.mailbox.lock().unwrap().len();
    assert_eq!(mailbox_size, 3); // Should have 3 messages left
    
    // Test selective receive for bytes
    let pattern = MessagePattern::Type(MessageType::Bytes);
    let result = context.selective_receive(pattern, Some(Duration::from_millis(100))).unwrap();
    assert!(result.is_some());
    if let Some(MessagePayload::Bytes(bytes)) = result {
        assert_eq!(bytes, vec![1, 2, 3]);
    } else {
        panic!("Expected bytes message");
    }
    
    // Test timeout with non-existent pattern
    let pattern = MessagePattern::Text("nonexistent".to_string());
    let start = std::time::Instant::now();
    let result = context.selective_receive(pattern, Some(Duration::from_millis(50))).unwrap();
    let elapsed = start.elapsed();
    
    assert!(result.is_none());
    assert!(elapsed >= Duration::from_millis(45)); // Allow some tolerance
    assert!(elapsed <= Duration::from_millis(100)); // Should not take too long
}

#[test]
fn test_actor_linking_lifecycle() {
    let pid1 = Pid::new();
    let pid2 = Pid::new();
    let pid3 = Pid::new();
    
    let context = ActorContext::new(pid1);
    
    // Test linking multiple actors
    context.link(pid2, LinkType::Bidirectional).unwrap();
    context.link(pid3, LinkType::Monitor).unwrap();
    
    let links = context.get_links();
    assert_eq!(links.len(), 2);
    
    // Verify link details
    let link_to_pid2 = links.iter().find(|l| l.linked_pid == pid2).unwrap();
    assert_eq!(link_to_pid2.link_type, LinkType::Bidirectional);
    
    let link_to_pid3 = links.iter().find(|l| l.linked_pid == pid3).unwrap();
    assert_eq!(link_to_pid3.link_type, LinkType::Monitor);
    
    // Test unlinking
    context.unlink(pid2).unwrap();
    let links = context.get_links();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].linked_pid, pid3);
    
    // Test unlinking non-existent link (should not error)
    context.unlink(Pid::new()).unwrap();
    let links = context.get_links();
    assert_eq!(links.len(), 1); // Should still have the link to pid3
}

#[test]
fn test_actor_monitoring_lifecycle() {
    let pid1 = Pid::new();
    let pid2 = Pid::new();
    let pid3 = Pid::new();
    
    let context = ActorContext::new(pid1);
    
    // Test monitoring multiple actors
    let monitor_ref1 = context.monitor(pid2).unwrap();
    let monitor_ref2 = context.monitor(pid3).unwrap();
    
    // Verify monitor references are unique
    assert_ne!(monitor_ref1, monitor_ref2);
    
    let monitors = context.get_monitors();
    assert_eq!(monitors.len(), 2);
    
    // Verify monitor details
    let monitor_for_pid2 = monitors.iter().find(|m| m.monitored_pid == pid2).unwrap();
    assert_eq!(monitor_for_pid2.monitor_ref, monitor_ref1);
    
    let monitor_for_pid3 = monitors.iter().find(|m| m.monitored_pid == pid3).unwrap();
    assert_eq!(monitor_for_pid3.monitor_ref, monitor_ref2);
    
    // Test demonitoring
    context.demonitor(monitor_ref1).unwrap();
    let monitors = context.get_monitors();
    assert_eq!(monitors.len(), 1);
    assert_eq!(monitors[0].monitored_pid, pid3);
    
    // Test demonitoring non-existent monitor (should not error)
    let fake_ref = MonitorRef::new();
    context.demonitor(fake_ref).unwrap();
    let monitors = context.get_monitors();
    assert_eq!(monitors.len(), 1); // Should still have the monitor for pid3
}

#[test]
fn test_enhanced_actor_with_context() {
    let pid = Pid::new();
    let initial_state = 0i32;
    
    // Create a behavior that uses the actor context
    let behavior = |state: &mut i32, message: MessagePayload| -> RuntimeResult<()> {
        match message {
            MessagePayload::Text(cmd) => {
                match cmd.as_str() {
                    "increment" => *state += 1,
                    "decrement" => *state -= 1,
                    "reset" => *state = 0,
                    _ => return Err(ream::error::RuntimeError::InvalidMessage(cmd)),
                }
            }
            MessagePayload::Data(data) => {
                if let Some(n) = data.as_i64() {
                    *state += n as i32;
                } else {
                    return Err(ream::error::RuntimeError::InvalidMessage("Expected number".to_string()));
                }
            }
            _ => return Err(ream::error::RuntimeError::InvalidMessage("Unsupported message type".to_string())),
        }
        Ok(())
    };
    
    let mut actor = Actor::new(pid, initial_state, behavior);
    
    // Test that actor has context
    assert!(actor.get_context().is_some());
    
    // Test actor functionality
    assert_eq!(*actor.state(), 0);
    
    // Test message processing
    actor.receive(MessagePayload::Text("increment".to_string())).unwrap();
    assert_eq!(*actor.state(), 1);
    
    actor.receive(MessagePayload::Data(serde_json::Value::Number(5.into()))).unwrap();
    assert_eq!(*actor.state(), 6);
    
    // Test linking through actor
    let target_pid = Pid::new();
    actor.link_to(target_pid, LinkType::Bidirectional).unwrap();
    
    if let Some(context) = actor.get_context() {
        let links = context.get_links();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].linked_pid, target_pid);
        assert_eq!(links[0].link_type, LinkType::Bidirectional);
    }
    
    // Test monitoring through actor
    let monitor_ref = actor.monitor_actor(target_pid).unwrap();
    assert!(monitor_ref.0 > 0);
    
    if let Some(context) = actor.get_context() {
        let monitors = context.get_monitors();
        assert_eq!(monitors.len(), 1);
        assert_eq!(monitors[0].monitored_pid, target_pid);
        assert_eq!(monitors[0].monitor_ref, monitor_ref);
    }
    
    // Test restart functionality
    actor.restart().unwrap();
    assert_eq!(*actor.state(), 0); // Should be reset to initial state
    
    // Context should be cleared after restart
    if let Some(context) = actor.get_context() {
        assert_eq!(context.mailbox.lock().unwrap().len(), 0);
    }
}

#[test]
fn test_system_message_handling() {
    let pid = Pid::new();
    let initial_state = 0i32;
    let behavior = |_state: &mut i32, _message: MessagePayload| -> RuntimeResult<()> { Ok(()) };
    
    let mut actor = Actor::new(pid, initial_state, behavior);
    
    // Test handling different system messages
    let down_msg = SystemMessage::Down { 
        pid: Pid::new(), 
        reason: "normal_exit".to_string() 
    };
    actor.handle_system_message(down_msg).unwrap();
    
    let link_msg = SystemMessage::Link { 
        from: Pid::new(), 
        link_type: LinkType::Bidirectional 
    };
    actor.handle_system_message(link_msg).unwrap();
    
    let unlink_msg = SystemMessage::Unlink { 
        from: Pid::new() 
    };
    actor.handle_system_message(unlink_msg).unwrap();
    
    let monitor_msg = SystemMessage::Monitor { 
        from: Pid::new(), 
        monitor_ref: MonitorRef::new() 
    };
    actor.handle_system_message(monitor_msg).unwrap();
    
    let demonitor_msg = SystemMessage::Demonitor { 
        monitor_ref: MonitorRef::new() 
    };
    actor.handle_system_message(demonitor_msg).unwrap();
}

#[test]
fn test_monitor_ref_uniqueness() {
    // Test that monitor references are unique
    let ref1 = MonitorRef::new();
    let ref2 = MonitorRef::new();
    let ref3 = MonitorRef::new();
    
    assert_ne!(ref1, ref2);
    assert_ne!(ref2, ref3);
    assert_ne!(ref1, ref3);
    
    // Test that references are incrementing
    assert!(ref2.0 > ref1.0);
    assert!(ref3.0 > ref2.0);
}
