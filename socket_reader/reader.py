import socket

def main():
    # Define the server address and port
    server_address = "127.69.42.0"
    server_port = 1412

    # Create a socket object
    client_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)

    try:
        # Connect to the server
        client_socket.connect((server_address, server_port))
        print(f"Connected to {server_address}:{server_port}")

        # Start reading data from the socket
        while True:
            data = client_socket.recv(1024)  # Receive up to 1024 bytes
            if not data:
                print("Connection closed by the server.")
                break

            print(f"Received: {data.decode('utf-8')}")

    except Exception as e:
        print(f"An error occurred: {e}")

    finally:
        # Close the socket
        client_socket.close()
        print("Connection closed.")

if __name__ == "__main__":
    main()
