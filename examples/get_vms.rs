use hyperv_rs::Hyperv;

fn main () {
    println!("Getting list of VMs on this machine...");
    match Hyperv::get_vms() {
        Ok(vms) => {
            print!("Got {} VMs", vms.len());
            if !vms.is_empty() {
                println!("");
                for vm in vms {
                println!("Id: {}, Name: {}", vm.id, vm.name);
                }
            }
        },
        Err(e) => println!("Error occured: {}", e),
    }
}