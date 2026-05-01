#![cfg(not(target_os = "windows"))]

use std::sync::Arc;

use anyhow::Result;
use thinwedge_core::ThinWedgeThread;
use thinwedge_protocol::protocol::EventMsg;
use thinwedge_protocol::protocol::Op;
use thinwedge_protocol::protocol::UndoCompletedEvent;
use core_test_support::test_thinwedge::TestThinWedgeHarness;
use core_test_support::test_thinwedge::test_thinwedge;
use core_test_support::wait_for_event_match;
use pretty_assertions::assert_eq;

async fn undo_harness() -> Result<TestThinWedgeHarness> {
    TestThinWedgeHarness::with_builder(test_thinwedge().with_model("gpt-5.4")).await
}

async fn invoke_undo(thinwedge: &Arc<ThinWedgeThread>) -> Result<UndoCompletedEvent> {
    thinwedge.submit(Op::Undo).await?;
    let event = wait_for_event_match(thinwedge, |msg| match msg {
        EventMsg::UndoCompleted(done) => Some(done.clone()),
        _ => None,
    })
    .await;
    Ok(event)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn undo_reports_feature_removal() -> Result<()> {
    let harness = undo_harness().await?;
    let thinwedge = Arc::clone(&harness.test().thinwedge);

    let event = invoke_undo(&thinwedge).await?;

    assert!(!event.success, "expected undo to fail");
    assert_eq!(
        event.message.as_deref(),
        Some("Undo is no longer available.")
    );

    Ok(())
}
