use std::future::Future;
use std::net::SocketAddr;
use std::rc::Rc;
use std::time::Duration;

use tokio::net::TcpListener;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Notify;

use crate::messages::gateway::ToGatewayMessage;
use crate::scheduler::scheduler::scheduler_loop;
use crate::server::comm::CommSenderRef;
use crate::server::core::CoreRef;

pub async fn server_start(
    listen_address: SocketAddr,
    msd: Duration,
    client_sender: UnboundedSender<ToGatewayMessage>,
    panic_on_worker_lost: bool,
) -> crate::Result<(
    CoreRef,
    CommSenderRef,
    impl Future<Output = crate::Result<()>>,
)> {
    log::debug!("Waiting for workers on {:?}", listen_address);
    let listener = TcpListener::bind(listen_address).await?;
    let listener_port = listener.local_addr().unwrap().port();
    /*let (comm, scheduler_sender, scheduler_receiver) = prepare_scheduler_comm();
    let scheduler_thread = start_scheduler(comm, scheduler_builder, msd);*/

    let scheduler_wakeup = Rc::new(Notify::new());

    let comm_ref = CommSenderRef::new(
        scheduler_wakeup.clone(),
        client_sender,
        panic_on_worker_lost,
    );
    let core_ref = CoreRef::new();
    core_ref.get_mut().set_worker_listen_port(listener_port);

    //let scheduler = observe_scheduler(core_ref.clone(), comm_ref.clone(), scheduler_receiver);
    let connections =
        crate::server::rpc::connection_initiator(listener, core_ref.clone(), comm_ref.clone());

    let scheduler = scheduler_loop(core_ref.clone(), comm_ref.clone(), scheduler_wakeup, msd);

    let future = async move {
        tokio::select! {
            () = scheduler => {},
            r = connections => r ?,
        };
        log::debug!("Waiting for scheduler to shut down...");
        //scheduler_thread.join().expect("Scheduler thread failed");
        log::info!("tako ends");
        Ok(())
    };

    Ok((core_ref, comm_ref, future))
}
