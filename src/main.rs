mod server;

use std::{io::{Read,Write,Result}};
use tokio::{net::{TcpListener, TcpStream}, io::{AsyncWriteExt, AsyncReadExt}};
use std::str;
use server:Server;

