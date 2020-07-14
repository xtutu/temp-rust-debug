use std::num::Wrapping;

pub fn calc_sum(data: &[u8]) -> u16 {
    let mut server_sum: i32 = 0;
    let mut length = data.len();
    let mut index = 0;
    // log::debug!("{:?}", data);
    while length > 1 {
        server_sum = server_sum + (Wrapping(data[index] as i32) << 8).0 + Wrapping(data[index + 1] as i32).0;
        // log::debug!("{}, {} {} {}", server_sum, data[index] ,Wrapping (data[index] as i32)<<8, data[index+1]);
        index += 2;
        length -= 2;
    }
    if length > 0 {
        server_sum += data[index] as i32;
    }
    server_sum += server_sum >> 16;

    // log::debug!("{}", server_sum);
    server_sum = !server_sum;
    // log::debug!("{}", server_sum as u16);
    return server_sum as u16;
}


pub fn process_data(data: &mut [u8]) {
    let x_or_bit = 106;
    for elem in data.iter_mut() {
        *elem ^= x_or_bit
    }
}
