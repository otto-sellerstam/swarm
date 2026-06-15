from anyio import connect_tcp, run

async def main():
    async with await connect_tcp("192.168.0.28", 8000) as client:
        await client.send(b"off\n")
        response = await client.receive()
        print(response)

run(main)